use crate::node::*;
use crate::service::osc::OscService;
use crate::service::websocket::WSService;

use petgraph::stable_graph::{NodeIndex, StableGraph, WalkNeighbors};
use rosc::{OscMessage, OscPacket};
use serde::{ser::SerializeMap, Serialize, Serializer};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

const NS_CHANGE_LEN: usize = 1024;

type Graph = StableGraph<NodeWrapper, ()>;

pub(crate) struct RootInner {
    name: Option<String>,
    graph: Graph,
    root: NodeIndex,
    //for fast lookup by full path
    index_map: HashMap<String, NodeIndex>,
    ns_change_send: Option<SyncSender<NamespaceChange>>, //TODO vec?
}

/// The root of an OSCQuery tree.
pub struct Root {
    inner: Arc<RwLock<RootInner>>,
}

pub(crate) struct NodeWrapper {
    pub(crate) full_path: String,
    pub(crate) node: Node,
}

pub(crate) struct NodeSerializeWrapper<'a> {
    node: &'a NodeWrapper,
    graph: &'a Graph,
    neighbors: WalkNeighbors<u32>,
    param: Option<NodeQueryParam>,
}

struct NodeSerializeContentsWrapper<'a> {
    graph: &'a Graph,
    neighbors: WalkNeighbors<u32>,
}

/// A handle for a node, to be used for triggering, adding children and/or removing.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct NodeHandle(NodeIndex);

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum NamespaceChange {
    PathAdded(String),
    PathRemoved(String),
}

impl Root {
    pub fn new(name: Option<String>) -> Self {
        let inner = Arc::new(RwLock::new(RootInner::new(name)));
        Self { inner }
    }

    pub fn spawn_osc<A: ToSocketAddrs>(&self, osc_addrs: A) -> Result<OscService, std::io::Error> {
        Ok(OscService::new(self.inner.clone(), osc_addrs)?)
    }

    pub fn spawn_ws<A: tokio::net::ToSocketAddrs>(
        &self,
        ws_addrs: A,
    ) -> Result<WSService, std::io::Error> {
        Ok(WSService::new(self.inner.clone(), ws_addrs)?)
    }

    pub fn name(&self) -> Option<String> {
        if let Ok(inner) = self.read_locked() {
            inner.name()
        } else {
            None
        }
    }

    fn write_locked(&self) -> Result<RwLockWriteGuard<RootInner>, &'static str> {
        self.inner.write().or_else(|_| Err("poisoned lock"))
    }

    fn read_locked(&self) -> Result<RwLockReadGuard<RootInner>, &'static str> {
        self.inner.read().or_else(|_| Err("poisoned lock"))
    }

    fn add(
        &self,
        node: Node,
        parent_index: Option<NodeIndex>,
    ) -> Result<NodeHandle, (Node, &'static str)> {
        match self.write_locked() {
            Ok(mut inner) => inner.add(node, parent_index),
            Err(s) => Err((node, s)),
        }
    }

    ///add node to the graph at the root or as a child of the given parent
    pub fn add_node(
        &self,
        node: Node,
        parent: Option<NodeHandle>,
    ) -> Result<NodeHandle, (Node, &'static str)> {
        let index = match parent {
            Some(NodeHandle(i)) => Some(i),
            None => None,
        };
        self.add(node, index)
    }

    ///Remove the node at the handle returns it and any children if found
    ///leafs come first in returned vector
    pub fn rm_node(&self, handle: NodeHandle) -> Result<Vec<Node>, (NodeHandle, &'static str)> {
        match self.write_locked() {
            Ok(mut inner) => inner.rm(handle.0).map_err(|e| (handle, e.1)),
            Err(s) => Err((handle, s)),
        }
    }

    pub(crate) fn serialize_node<F, S>(
        &self,
        path: &str,
        param: Option<NodeQueryParam>,
        f: F,
    ) -> Result<S::Ok, S::Error>
    where
        F: FnOnce(Option<&NodeSerializeWrapper>) -> Result<S::Ok, S::Error>,
        S: Serializer,
    {
        self.read_locked()
            .expect("failed to read lock")
            .serialize_node::<F, S>(path, param, f)
    }
}

impl Serialize for Root {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let root = self.read_locked().expect("failed to read lock");
        serializer.serialize_some(&*root)
    }
}

impl RootInner {
    pub fn new(name: Option<String>) -> Self {
        let mut graph = StableGraph::default();
        let root = graph.add_node(NodeWrapper {
            full_path: "/".to_string(),
            node: Node::Container(Container {
                address: "".to_string(), //invalid, but unchecked by default access
                description: Some("root node".to_string()),
            }),
        });
        let mut index_map = HashMap::new();
        index_map.insert("/".to_string(), root);
        Self {
            name,
            graph,
            root,
            index_map,
            ns_change_send: None,
        }
    }

    pub(crate) fn ns_change_recv(&mut self) -> Option<Receiver<NamespaceChange>> {
        if self.ns_change_send.is_some() {
            None
        } else {
            let (send, recv) = sync_channel(NS_CHANGE_LEN);
            self.ns_change_send = Some(send);
            Some(recv)
        }
    }

    pub fn with_node_at_handle<F, R>(&self, handle: &NodeHandle, f: F) -> R
    where
        F: Fn(Option<&NodeWrapper>) -> R,
    {
        f(self.graph.node_weight(handle.0))
    }

    pub fn with_node_at_path<F, R>(&self, path: &str, f: F) -> R
    where
        F: Fn(Option<&NodeWrapper>) -> R,
    {
        f(if let Some(index) = self.index_map.get(path) {
            self.graph.node_weight(*index)
        } else {
            None
        })
    }

    fn handle_osc_msg(&self, msg: &OscMessage, addr: Option<SocketAddr>, time: Option<(u32, u32)>) {
        self.with_node_at_path(&msg.addr, |node| {
            if let Some(node) = node {
                node.node.osc_update(&msg.args, addr, time);
            }
        });
    }

    pub fn handle_osc_packet(
        &self,
        packet: &OscPacket,
        addr: Option<SocketAddr>,
        time: Option<(u32, u32)>,
    ) {
        match packet {
            OscPacket::Message(msg) => self.handle_osc_msg(&msg, addr, time),
            OscPacket::Bundle(bundle) => {
                for p in bundle.content.iter() {
                    self.handle_osc_packet(p, addr.clone(), Some(bundle.timetag));
                }
            }
        };
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub(crate) fn serialize_node<F, S>(
        &self,
        path: &str,
        param: Option<NodeQueryParam>,
        f: F,
    ) -> Result<S::Ok, S::Error>
    where
        F: FnOnce(Option<&NodeSerializeWrapper>) -> Result<S::Ok, S::Error>,
        S: Serializer,
    {
        match self.index_map.get(path) {
            Some(index) => match self.graph.node_weight(index.clone()) {
                Some(node) => f(Some(&NodeSerializeWrapper {
                    node,
                    graph: &self.graph,
                    neighbors: self.graph.neighbors(*index).detach(),
                    param,
                })),
                None => f(None),
            },
            None => f(None),
        }
    }

    pub fn rm(&mut self, index: NodeIndex) -> Result<Vec<Node>, (NodeIndex, &'static str)> {
        let mut children = self.graph.neighbors(index).detach();
        let mut v = Vec::new();
        while let Some(index) = children.next_node(&self.graph) {
            v.append(&mut self.rm(index).expect("child should be in graph"));
        }
        match self.graph.remove_node(index) {
            Some(node) => {
                self.index_map.remove(&node.full_path);
                v.push(node.node);
                if let Some(ns_change_send) = &self.ns_change_send {
                    let _ = ns_change_send
                        .try_send(NamespaceChange::PathRemoved(node.full_path.clone()));
                }
                Ok(v)
            }
            None => Err((index, &"node at handle not in graph")),
        }
    }

    pub fn add(
        &mut self,
        node: Node,
        parent_index: Option<NodeIndex>,
    ) -> Result<NodeHandle, (Node, &'static str)> {
        let (parent_index, full_path) = if let Some(parent_index) = parent_index {
            if let Some(parent) = self.graph.node_weight(parent_index.clone()) {
                Ok((parent_index, parent.full_path.clone()))
            } else {
                return Err((node, "parent not in graph"));
            }
        } else {
            Ok((self.root, "".to_string()))
        }?;

        //compute the full path
        let full_path = format!("{}/{}", full_path, node.address());
        let node = NodeWrapper {
            node,
            full_path: full_path.clone(),
        };

        //actually add
        let index = self.graph.add_node(node);
        self.index_map.insert(full_path.clone(), index);
        let _ = self.graph.add_edge(parent_index, index, ());
        if let Some(ns_change_send) = &self.ns_change_send {
            let _ = ns_change_send.try_send(NamespaceChange::PathAdded(full_path));
        }
        Ok(NodeHandle(index))
    }
}

impl Serialize for RootInner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.serialize_node::<_, S>(&"/", None, move |n| {
            serializer.serialize_some(n.expect("root must be in graph"))
        })
    }
}

impl<'a> Serialize for NodeSerializeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let n = &self.node.node;
        match self.param {
            None => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("ACCESS".into(), &n.access())?;
                if let Some(d) = n.description() {
                    m.serialize_entry("DESCRIPTION".into(), d)?;
                }
                m.serialize_entry("FULL_PATH".into(), &(self.node.full_path))?;
                match n {
                    Node::Get(..) | Node::GetSet(..) => {
                        m.serialize_entry("VALUE".into(), &NodeValueWrapper(n))?;
                    }
                    _ => (),
                };
                match n {
                    Node::Container(..) => {
                        m.serialize_entry(
                            "CONTENTS".into(),
                            &NodeSerializeContentsWrapper {
                                graph: self.graph,
                                neighbors: self.neighbors.clone(),
                            },
                        )?;
                    }
                    _ => {
                        if let Some(t) = n.type_string() {
                            m.serialize_entry("TYPE".into(), &t)?;
                        }
                        m.serialize_entry("RANGE".into(), &NodeRangeWrapper(n))?;
                        m.serialize_entry("CLIPMODE".into(), &NodeClipModeWrapper(n))?;
                        m.serialize_entry("UNIT".into(), &NodeUnitWrapper(n))?;
                    }
                };
                m.end()
            }
            Some(NodeQueryParam::Access) => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("ACCESS".into(), &n.access())?;
                m.end()
            }
            Some(NodeQueryParam::Description) => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("DESCRIPTION".into(), n.description())?;
                m.end()
            }
            Some(NodeQueryParam::Value) => match n {
                Node::Get(..) | Node::GetSet(..) => {
                    let mut m = serializer.serialize_map(None)?;
                    m.serialize_entry("VALUE".into(), &NodeValueWrapper(n))?;
                    m.end()
                }
                _ => serializer.serialize_none(),
            },
            Some(NodeQueryParam::Range) => match n {
                Node::Container(..) => serializer.serialize_none(),
                _ => {
                    let mut m = serializer.serialize_map(None)?;
                    m.serialize_entry("RANGE".into(), &NodeRangeWrapper(n))?;
                    m.end()
                }
            },
            Some(NodeQueryParam::ClipMode) => match n {
                Node::Container(..) => serializer.serialize_none(),
                _ => {
                    let mut m = serializer.serialize_map(None)?;
                    m.serialize_entry("CLIPMODE".into(), &NodeClipModeWrapper(n))?;
                    m.end()
                }
            },
            Some(NodeQueryParam::Type) => match n {
                Node::Container(..) => serializer.serialize_none(),
                _ => {
                    let mut m = serializer.serialize_map(None)?;
                    m.serialize_entry("TYPE".into(), &n.type_string())?;

                    m.end()
                }
            },
            Some(NodeQueryParam::Unit) => match n {
                Node::Container(..) => serializer.serialize_none(),
                _ => {
                    let mut m = serializer.serialize_map(None)?;
                    m.serialize_entry("UNIT".into(), &NodeUnitWrapper(n))?;
                    m.end()
                }
            },
        }
    }
}

impl<'a> Serialize for NodeSerializeContentsWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_map(None)?;
        let mut neighbors = self.neighbors.clone();
        while let Some(index) = neighbors.next_node(self.graph) {
            if let Some(node) = self.graph.node_weight(index) {
                let w = NodeSerializeWrapper {
                    node: &node,
                    graph: self.graph,
                    neighbors: self.graph.neighbors(index).detach(),
                    param: None,
                };
                m.serialize_entry(&node.node.address(), &w)?;
            }
        }
        m.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::param::*;

    use crate::value::*;
    use ::atomic::Atomic;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn basic_expectations() {
        let root = Root::new(Some("test".into()));

        let c = Container::new("foo".into(), Some("description of foo".into()));
        assert!(c.is_ok());

        let res = root.add_node(c.unwrap().into(), None);
        assert!(res.is_ok());

        let chandle = res.unwrap();

        let c = Container::new("bar".into(), Some("description of foo".into()));
        assert!(c.is_ok());

        let res = root.add_node(c.unwrap().into(), Some(chandle));
        assert!(res.is_ok());

        let a = Arc::new(Atomic::new(2084i32));
        let m = crate::node::Get::new(
            "baz".into(),
            None,
            vec![ParamGet::Int(ValueBuilder::new(a.clone() as _).build())],
        );

        //can add a method
        let res = root.add_node(m.unwrap().into(), Some(chandle));
        assert!(res.is_ok());

        let mhandle = res.unwrap();

        //okay to add method to method
        let m = crate::node::GetSet::new(
            "biz".into(),
            None,
            vec![ParamGetSet::Int(ValueBuilder::new(a.clone() as _).build())],
            None,
        );

        let res = root.add_node(m.unwrap().into(), Some(mhandle));
        assert!(res.is_ok());

        //can remove a method
        let handle = res.unwrap();
        let res = root.rm_node(handle.clone());
        assert!(res.is_ok());
        let v = res.unwrap();
        assert_eq!(1, v.len());
        //second attempt gives error
        assert!(root.rm_node(handle.clone()).is_err());

        //can remove the top
        let res = root.rm_node(chandle);
        assert!(res.is_ok());
        let v = res.unwrap();
        assert_eq!(3, v.len());

        //come out with leaf first
        assert_eq!(&"baz", v[0].address());
        assert_eq!(&"bar", v[1].address());
        assert_eq!(&"foo", v[2].address());
    }

    #[test]
    fn is_send_and_sync() {
        let root = Arc::new(Root::new(None));

        let c = Container::new("foo".into(), Some("description of foo".into()));
        assert!(c.is_ok());

        let a = Arc::new(Atomic::new(2084i32));
        let m = crate::node::Set::new(
            "baz".into(),
            None,
            vec![ParamSet::Int(ValueBuilder::new(a.clone() as _).build())],
            None,
        );

        let r = root.clone();
        let h = thread::spawn(move || {
            let res = r.add_node(c.unwrap().into(), None);
            assert!(res.is_ok());

            let c = Container::new("bar".into(), None);
            assert!(c.is_ok());
            let res = r.add_node(c.unwrap().into(), Some(res.unwrap()));
            assert!(res.is_ok());

            let res = r.add_node(m.unwrap().into(), Some(res.unwrap()));
            assert!(res.is_ok());
        });
        let c = Container::new("bar".into(), None);
        assert!(c.is_ok());
        let res = root.add_node(c.unwrap().into(), None);
        assert!(res.is_ok());

        assert!(h.join().is_ok());
    }

    use serde_json::json;

    #[test]
    fn serialize() {
        let root = Arc::new(Root::new(Some("test".into())));

        let c = Container::new("foo".into(), Some("description of foo".into()));
        assert!(c.is_ok());
        let res = root.add_node(c.unwrap().into(), None);
        assert!(res.is_ok());

        let a = Arc::new(Atomic::new(2084i32));
        let m = crate::node::Get::new(
            "bar".into(),
            None,
            vec![ParamGet::Int(
                ValueBuilder::new(a.clone() as _)
                    .with_unit("distance.m".into())
                    .build(),
            )],
        );

        let res = root.add_node(m.unwrap().into(), Some(res.unwrap()));
        assert!(res.is_ok());

        let j = serde_json::to_value(root);
        assert!(j.is_ok());
        assert_eq!(
            j.unwrap(),
            json!({
                "ACCESS": 0,
                "DESCRIPTION": "root node",
                "FULL_PATH": "/",
                "CONTENTS": {
                    "foo": {
                        "ACCESS": 0,
                        "DESCRIPTION": "description of foo",
                        "FULL_PATH": "/foo",
                        "CONTENTS": {
                            "bar": {
                                "ACCESS": 1,
                                "FULL_PATH": "/foo/bar",
                                "VALUE": [2084],
                                "UNIT": ["distance.m"],
                                "TYPE": "i",
                                "RANGE": [{}],
                                "CLIPMODE": ["none"]
                            }
                        }
                    }
                }
            })
            .clone()
        );
    }
}
