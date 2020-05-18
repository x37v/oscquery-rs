use crate::root::Root;
use crate::service::*;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

/// A batteries included ease of use wrapper for the various services that make osc query.
pub struct OscQueryServer {
    root: Arc<Root>,
    osc: osc::OscService,
    ws: websocket::WSService,
    http: http::HttpService,
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
            http,
        })
    }
}
