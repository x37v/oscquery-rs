use crate::node::*;
use crate::service::osc::OscService;
use petgraph::stable_graph::{NodeIndex, StableGraph, WalkNeighbors};
use rosc::{OscBundle, OscMessage, OscPacket};
use serde::{ser::SerializeMap, Serialize, Serializer};
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

type Graph = StableGraph<NodeWrapper, ()>;

pub(crate) struct RootInner {
    name: Option<String>,
    graph: Graph,
    root: NodeIndex,
    //for fast lookup by full path
    index_map: HashMap<String, NodeIndex>,
}

/// The root of an OSCQuery tree.
pub struct Root {
    inner: Arc<RwLock<RootInner>>,
}

pub(crate) struct NodeWrapper {
    full_path: String,
    node: Node,
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

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum NodeHandle {
    Container(NodeIndex),
    Method(NodeIndex),
}

impl Root {
    pub fn new(name: Option<String>) -> Self {
        let inner = Arc::new(RwLock::new(RootInner::new(name)));
        Self { inner }
    }

    pub fn spawn_osc<A: ToSocketAddrs>(&self, osc_addrs: A) -> Result<OscService, std::io::Error> {
        Ok(OscService::new(self.inner.clone(), osc_addrs)?)
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

    pub fn add_node(
        &self,
        node: Node,
        parent: Option<NodeHandle>,
    ) -> Result<NodeHandle, (Node, &'static str)> {
        match parent {
            Some(NodeHandle::Container(i)) => self.add(node, Some(i)),
            Some(NodeHandle::Method(_)) => Err((node, "cannot add node to a method node")),
            None => self.add(node, None),
        }
    }

    //TODO remove_node
    //ADD method with /long/path/to/leaf so we don't have to add each individual container

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
        }
    }

    fn with_node_at_handle<F>(&self, handle: &NodeHandle, f: F)
    where
        F: Fn(Option<&NodeWrapper>),
    {
        let index = match handle {
            NodeHandle::Container(i) => i,
            NodeHandle::Method(i) => i,
        };
        f(self.graph.node_weight(*index));
    }

    fn with_node_at_path<F>(&self, path: &str, f: F)
    where
        F: Fn(Option<&NodeWrapper>),
    {
        f(if let Some(index) = self.index_map.get(path) {
            self.graph.node_weight(*index)
        } else {
            None
        });
    }

    fn handle_osc_msg(&self, msg: &OscMessage) {
        self.with_node_at_path(&msg.addr, |node| {
            if let Some(node) = node {
                node.node.osc_update(&msg.args);
            }
        });
    }

    pub fn handle_osc_packet(&self, packet: &OscPacket) {
        match packet {
            OscPacket::Message(msg) => self.handle_osc_msg(&msg),
            OscPacket::Bundle(bundle) => {
                for p in bundle.content.iter() {
                    //TODO something with time stamp?
                    self.handle_osc_packet(p);
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

    pub fn add(
        &mut self,
        node: Node,
        parent_index: Option<NodeIndex>,
    ) -> Result<NodeHandle, (Node, &'static str)> {
        let cont = match node {
            Node::Container(_) => true,
            _ => false,
        };
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
        self.index_map.insert(full_path, index);
        let _ = self.graph.add_edge(parent_index, index, ());
        Ok(if cont {
            NodeHandle::Container(index)
        } else {
            NodeHandle::Method(index)
        })
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

        //fails to add method to method
        let m = crate::node::Get::new(
            "biz".into(),
            None,
            vec![ParamGet::Int(ValueBuilder::new(a.clone() as _).build())],
        );

        let res = root.add_node(m.unwrap().into(), Some(mhandle));
        assert!(res.is_err());

        //but can then add to root
        let res = root.add_node(res.err().unwrap().0, None);
        assert!(res.is_ok());
    }

    #[test]
    fn is_send_and_sync() {
        let root = Arc::new(Root::new(None));

        let c = Container::new("foo".into(), Some("description of foo".into()));
        assert!(c.is_ok());

        let a = Arc::new(Atomic::new(2084i32));
        let m = crate::node::Get::new(
            "baz".into(),
            None,
            vec![ParamGet::Int(ValueBuilder::new(a.clone() as _).build())],
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
