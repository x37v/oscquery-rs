use crate::node::Node;
use crate::root::{NodeHandle, Root};
use crate::service::{http, osc, websocket};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

/// A batteries included ease of use wrapper for the various services that make osc query.
pub struct OscQueryServer {
    root: Arc<Root>,
    osc: osc::OscService,
    ws: websocket::WSService,
    _http: http::HttpService,
}

impl OscQueryServer {
    pub fn new<OA: ToSocketAddrs, WA: ToSocketAddrs>(
        server_name: Option<String>,
        http_addr: &SocketAddr,
        osc_addr: OA,
        ws_addr: WA,
    ) -> Result<Self, std::io::Error> {
        let root = Arc::new(Root::new(server_name));
        let osc = root.spawn_osc(osc_addr)?;
        let ws = root.spawn_ws(ws_addr)?;
        let http = http::HttpService::new(
            root.clone(),
            http_addr,
            Some(osc.local_addr().clone()),
            Some(ws.local_addr().clone()),
        );

        Ok(Self {
            root,
            osc,
            ws,
            _http: http,
        })
    }

    ///Add node to the graph at the root or as a child of the given parent
    pub fn add_node(
        &self,
        node: Node,
        parent: Option<NodeHandle>,
    ) -> Result<NodeHandle, (Node, &'static str)> {
        self.root.add_node(node, parent)
    }

    ///Remove the node at the handle returns it and any children if found.
    ///
    ///Leaves come first in returned vector.
    pub fn rm_node(&self, handle: NodeHandle) -> Result<Vec<Node>, (NodeHandle, &'static str)> {
        self.root.rm_node(handle)
    }

    ///Get the OSC service's bound address.
    pub fn osc_local_addr(&self) -> &SocketAddr {
        self.osc.local_addr()
    }

    ///Get the websocket service's bound address.
    pub fn ws_local_addr(&self) -> &SocketAddr {
        self.ws.local_addr()
    }

    ///Trigger a send (if possible) for the node at the given handle.
    ///
    ///Returns true if there was a node at the handle that could be and was triggered.
    pub fn trigger(&self, handle: NodeHandle) -> bool {
        if let Some(msg) = self.osc.trigger(handle) {
            self.ws.send(msg);
            true
        } else {
            false
        }
    }

    ///Trigger a send (if possible) for the node at the given path.
    ///
    ///Returns true if there was a node at the path that could be and was triggered.
    pub fn trigger_path(&self, path: &str) -> bool {
        if let Some(msg) = self.osc.trigger_path(path) {
            self.ws.send(msg);
            true
        } else {
            false
        }
    }
}
