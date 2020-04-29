use crate::root::Root;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Response, Server};
use std::sync::Arc;

pub struct ServiceHandle {
    tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ServiceHandle {
    pub fn new(root: Arc<Root>) -> Self {
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
                let make_service = make_service_fn(move |_| {
                    let root = root.clone();
                    async move {
                        Ok::<_, Error>(service_fn(move |_req| {
                            let root = root.clone();
                            async move {
                                let s = serde_json::to_value(root).unwrap().to_string();
                                Ok::<_, Error>(Response::new(Body::from(s)))
                            }
                        }))
                    }
                });
                let server = Server::bind(&([127, 0, 0, 1], 3000).into()).serve(make_service);

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
