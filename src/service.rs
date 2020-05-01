use crate::root::Root;

use futures::future;
use hyper::service::{make_service_fn, service_fn, Service};
use hyper::{Body, Error, Request, Response, Server};
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

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let rsp = Response::builder();

        let uri = req.uri();
        if uri.path() != "/" {
            let body = Body::from(Vec::new());
            let rsp = rsp.status(404).body(body).unwrap();
            return future::ok(rsp);
        }

        let s = serde_json::to_value(self.root.clone()).unwrap().to_string();
        let body = Body::from(s);
        let rsp = rsp.status(200).body(body).unwrap();
        future::ok(rsp)
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
