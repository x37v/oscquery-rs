use crate::root::Root;
use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Request, Response, Server, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use std::convert::Infallible;

pub struct ServiceHandle {
    tx: tokio::sync::oneshot::Sender<()>,
}

impl ServiceHandle {
    pub fn new(root: Arc<Root>) -> Self {
        let root = root.clone();
        let make_service = make_service_fn(move |_| {
            let root = root.clone();
            async move {
                let root = root.clone();
                Ok::<_, Error>(service_fn(move |_req| {
                    let root = root.clone();
                    async move {
                        let s = serde_json::to_value(root.clone()).unwrap().to_string();
                        Ok::<_, Error>(Response::new(Body::from(s)))
                    }
                }))
            }
        });
        let server = Server::bind(&([127, 0, 0, 1], 3000).into()).serve(make_service);

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let graceful = server.with_graceful_shutdown(async {
            rx.await.ok();
        });

        std::thread::spawn(move || {
            let mut rt = tokio::runtime::Runtime::new().unwrap(); //todo ?
            rt.block_on(async {
                if let Err(e) = graceful.await {
                    eprintln!("server error: {}", e);
                }
            });
        });

        Self { tx }
    }
}
