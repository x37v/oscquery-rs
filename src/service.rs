use crate::node::NodeQueryParam;
use crate::root::Root;

use futures::future;
use hyper::service::Service;
use hyper::{header, Body, Method, Request, Response, Server};
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use std::task::{Context, Poll};

pub struct ServiceHandle {
    tx: Option<tokio::sync::oneshot::Sender<()>>,
}

struct Svc {
    root: Arc<Root>,
}

struct MakeSvc {
    root: Arc<Root>,
}

struct PathSerializeWrapper<'a> {
    root: Arc<Root>,
    path: &'a str,
    param: Option<NodeQueryParam>,
}

struct HostInfoWrapper {
    root: Arc<Root>,
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
pub struct Extensions {
    access: bool,
    value: bool,
    range: bool,
    description: bool,
    clipmode: bool,
    unit: bool,
    //TODO more
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
        }
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
        let e: Extensions = Default::default();
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
        })
    }
}

impl ServiceHandle {
    pub fn new(root: Arc<Root>, port: u16) -> Self {
        let root = root.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Builder::new()
                .basic_scheduler()
                .threaded_scheduler()
                .enable_all()
                .build()
                .expect("could not create runtime");
            rt.block_on(async {
                let server = Server::bind(&([127, 0, 0, 1], port).into()).serve(MakeSvc { root });

                let graceful = server.with_graceful_shutdown(async {
                    rx.await.ok();
                    println!("quitting");
                });

                if let Err(e) = graceful.await {
                    eprintln!("server error: {}", e);
                }
            });
        });
        Self { tx: Some(tx) }
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
    }
}
