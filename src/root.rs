use crate::node::*;
use petgraph::stable_graph::{NodeIndex, StableGraph, WalkNeighbors};
use serde::{ser::SerializeMap, Serialize, Serializer};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

type Graph = StableGraph<NodeWrapper, ()>;

struct RootInner {
    graph: Graph,
    root: NodeIndex,
}

pub struct Root {
    inner: RwLock<RootInner>,
}

struct NodeWrapper {
    full_path: String,
    node: Node,
}

struct NodeSerializeWrapper<'a> {
    node: &'a NodeWrapper,
    graph: &'a Graph,
    neighbors: WalkNeighbors<u32>,
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

impl Default for RootInner {
    fn default() -> Self {
        let mut graph = StableGraph::default();
        let root = graph.add_node(NodeWrapper {
            full_path: "/".to_string(),
            node: Node::Container(Container {
                address: "".to_string(), //invalid, but unchecked by default access
                description: Some("root node".to_string()),
            }),
        });
        Self { graph, root }
    }
}

impl Root {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(RootInner::default()),
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
        let cont = match node {
            Node::Container(_) => true,
            _ => false,
        };
        match self.write_locked() {
            Ok(mut inner) => {
                let (parent_index, full_path) = if let Some(parent_index) = parent_index {
                    if let Some(parent) = inner.graph.node_weight(parent_index.clone()) {
                        Ok((parent_index, parent.full_path.clone()))
                    } else {
                        return Err((node, "parent not in graph"));
                    }
                } else {
                    Ok((inner.root, "".to_string()))
                }?;

                //compute the full path
                let full_path = format!("{}/{}", full_path, node.address());
                let node = NodeWrapper { node, full_path };

                //actually add
                let index = inner.graph.add_node(node);
                let _ = inner.graph.add_edge(parent_index, index, ());
                Ok(if cont {
                    NodeHandle::Container(index)
                } else {
                    NodeHandle::Method(index)
                })
            }
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

impl Serialize for RootInner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let root = self
            .graph
            .node_weight(self.root)
            .expect("root must be in graph");
        serializer.serialize_some(&NodeSerializeWrapper {
            node: root,
            graph: &self.graph,
            neighbors: self.graph.neighbors(self.root).detach(),
        })
    }
}

impl<'a> Serialize for NodeSerializeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_map(None)?;
        let n = &self.node.node;
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
                m.serialize_entry("CLIP_MODE".into(), &NodeClipModeWrapper(n))?;
            }
        };

        m.end()
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
        let root = Root::new();

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
        let v = ParamGet::Int(ValueBuilder::new(a.clone() as _).build());
        let v = vec![v];
        let m = crate::node::Get::new("baz".into(), None, v.into_iter());

        //can add a method
        let res = root.add_node(m.unwrap().into(), Some(chandle));
        assert!(res.is_ok());

        let mhandle = res.unwrap();

        //fails to add method to method
        let v = ParamGet::Int(ValueBuilder::new(a.clone() as _).build());
        let v = vec![v];
        let m = crate::node::Get::new("biz".into(), None, v.into_iter());

        let res = root.add_node(m.unwrap().into(), Some(mhandle));
        assert!(res.is_err());

        //but can then add to root
        let res = root.add_node(res.err().unwrap().0, None);
        assert!(res.is_ok());
    }

    #[test]
    fn is_send_and_sync() {
        let root = Arc::new(Root::new());

        let c = Container::new("foo".into(), Some("description of foo".into()));
        assert!(c.is_ok());

        let a = Arc::new(Atomic::new(2084i32));
        let v = ParamGet::Int(ValueBuilder::new(a.clone() as _).build());
        let v = vec![v];
        let m = crate::node::Get::new("baz".into(), None, v.into_iter());

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
        let root = Arc::new(Root::new());

        let c = Container::new("foo".into(), Some("description of foo".into()));
        assert!(c.is_ok());
        let res = root.add_node(c.unwrap().into(), None);
        assert!(res.is_ok());

        let a = Arc::new(Atomic::new(2084i32));
        let v = ParamGet::Int(ValueBuilder::new(a.clone() as _).build());
        let v = vec![v];
        let m = crate::node::Get::new("bar".into(), None, v.into_iter());

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
                                "TYPE": "i",
                                "RANGE": [{}],
                                "CLIP_MODE": ["none"]
                            }
                        }
                    }
                }
            })
            .clone()
        );
    }
}
