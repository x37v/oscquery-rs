use crate::node::NodeQueryParam;
use crate::root::Root;

use futures::future;
use hyper::service::Service;
use hyper::{header, Body, Method, Request, Response, Server};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::task::{Context, Poll};

/// The http server service for OSCQuery http requests.
pub struct HttpService {
    tx: Option<tokio::sync::oneshot::Sender<()>>,
    addr: SocketAddr,
}

struct Svc {
    root: Arc<Root>,
    osc: Option<SocketAddr>,
    ws: Option<SocketAddr>,
}

struct MakeSvc {
    root: Arc<Root>,
    osc: Option<SocketAddr>,
    ws: Option<SocketAddr>,
}

struct PathSerializeWrapper<'a> {
    root: Arc<Root>,
    path: &'a str,
    param: Option<NodeQueryParam>,
}

struct HostInfoWrapper {
    root: Arc<Root>,
    osc: Option<SocketAddr>,
    ws: Option<SocketAddr>,
}

impl<'a> Serialize for PathSerializeWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.root
            .serialize_node::<_, S>(self.path, self.param, move |n| {
                if let Some(n) = n {
                    serializer.serialize_some(n)
                } else {
                    Err(serde::ser::Error::custom("path not in namespace"))
                }
            })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) struct Extensions {
    access: bool,
    value: bool,
    range: bool,
    description: bool,
    clipmode: bool,
    unit: bool,

    listen: bool,
    path_changed: bool,
    path_renamed: bool,
    path_added: bool,
    path_removed: bool,

    //TODO
    tags: bool,
    extended_type: bool,
    critical: bool,
    overloads: bool,
    html: bool,
}

impl Default for Extensions {
    fn default() -> Self {
        Self {
            access: true,
            value: true,
            range: true,
            description: true,
            clipmode: true,
            unit: true,

            listen: false,
            path_changed: false,
            path_renamed: false,
            path_added: false,
            path_removed: false,

            tags: false,
            extended_type: false,
            critical: false,
            overloads: false,
            html: false,
        }
    }
}

impl Extensions {
    pub(crate) fn with_ws(&mut self) {
        self.listen = true;
        self.path_added = true;
        self.path_removed = true;
    }
}

impl Serialize for HostInfoWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_map(None)?;
        if let Some(name) = self.root.name() {
            m.serialize_entry("NAME".into(), &name)?;
        }
        if let Some(addr) = &self.osc {
            //TODO TCP support?
            m.serialize_entry("OSC_TRANSPORT", &"UDP")?;
            m.serialize_entry("OSC_IP", &addr.ip())?;
            m.serialize_entry("OSC_PORT", &addr.port())?;
        }
        let mut e: Extensions = Default::default();
        if let Some(addr) = &self.ws {
            e.with_ws();
            m.serialize_entry("WS_IP", &addr.ip())?;
            m.serialize_entry("WS_PORT", &addr.port())?;
        }
        m.serialize_entry("EXTENSIONS".into(), &e)?;
        m.end()
    }
}

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let rsp = if req.method() == &Method::GET {
            let mut param: Option<NodeQueryParam> = None;
            if let Some(p) = req.uri().query() {
                if p == "HOST_INFO" {
                    let w = HostInfoWrapper {
                        root: self.root.clone(),
                        osc: self.osc.clone(),
                        ws: self.ws.clone(),
                    };
                    return future::ok(
                        Response::builder()
                            .status(200)
                            .body(Body::from(
                                serde_json::to_string(&w).expect("failed to HostInfoWrapper"),
                            ))
                            .unwrap(),
                    );
                } else {
                    let p: Result<NodeQueryParam, _> =
                        serde_json::from_value(serde_json::Value::String(p.to_string()));
                    match p {
                        Ok(p) => param = Some(p),
                        Err(e) => {
                            return future::ok(
                                Response::builder()
                                    .status(400)
                                    .body(Body::from(e.to_string()))
                                    .unwrap(),
                            );
                        }
                    };
                }
            };
            let s = PathSerializeWrapper {
                root: self.root.clone(),
                path: req.uri().path(),
                param,
            };
            //might be Null, in which case we should return 204
            if let Ok(s) = serde_json::to_value(&s) {
                Some(match s {
                    serde_json::Value::Null => Response::builder().status(204).body(Body::empty()),
                    _ => Response::builder()
                        .status(200)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(s.to_string())),
                })
            } else {
                None
            }
        } else {
            None
        }
        .unwrap_or(Response::builder().status(404).body(Body::from(Vec::new())));
        future::ok(rsp.expect("expected response"))
    }
}

impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(Svc {
            root: self.root.clone(),
            osc: self.osc.clone(),
            ws: self.ws.clone(),
        })
    }
}

impl HttpService {
    /// Construct a new http server.
    pub fn new(
        root: Arc<Root>,
        addr: &SocketAddr,
        osc: Option<SocketAddr>,
        ws: Option<SocketAddr>,
    ) -> Self {
        let root = root.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let addr = addr.clone();
        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Builder::new()
                .basic_scheduler()
                .threaded_scheduler()
                .enable_all()
                .build()
                .expect("could not create runtime");
            rt.block_on(async {
                let server = Server::bind(&addr).serve(MakeSvc { root, osc, ws });
                let graceful = server.with_graceful_shutdown(async {
                    rx.await.ok();
                    println!("quitting");
                });

                if let Err(e) = graceful.await {
                    eprintln!("server error: {}", e);
                }
            });
        });
        Self { tx: Some(tx), addr }
    }

    ///The the `SocketAddr` that the http service is bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.addr
    }
}

impl Drop for HttpService {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
    }
}
