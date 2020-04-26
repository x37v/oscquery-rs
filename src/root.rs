use crate::node::*;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use std::sync::{Mutex, MutexGuard};

struct RootInner {
    graph: StableGraph<NodeWrapper, ()>,
    root: NodeIndex,
}

pub struct Root {
    inner: Mutex<RootInner>,
}

struct NodeWrapper {
    full_path: String,
    node: Node,
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
            full_path: "".to_string(),
            node: Node::Container(Container {
                address: "".to_string(), //invalid, but unchecked by default access
                description: None,
            }),
        });
        Self { graph, root }
    }
}

impl Root {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RootInner::default()),
        }
    }

    fn locked(&self) -> Result<MutexGuard<RootInner>, &'static str> {
        self.inner.lock().or_else(|_| Err("poisoned lock"))
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
        match self.locked() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::*;
    use crate::param::*;
    use crate::value::atomic::*;
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
}
