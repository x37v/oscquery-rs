use ::atomic::Atomic;
use oscquery::param::*;
use oscquery::value::*;
use oscquery::OscQueryServer;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

fn main() -> Result<(), std::io::Error> {
    let root = OscQueryServer::new(
        Some("example".into()),
        &SocketAddr::from_str("127.0.0.1:3000").expect("failed to bind for http"),
        "127.0.0.1:0",
        "127.0.0.1:0",
    )?;

    println!(
        "http: {} osc: {} ws: {}",
        root.http_local_addr(),
        root.osc_local_addr(),
        root.ws_local_addr()
    );

    let c = oscquery::node::Container::new("foo".into(), Some("description of foo".into()));
    assert!(c.is_ok());
    let res = root.add_node(c.unwrap().into(), None);
    assert!(res.is_ok());

    let a = Arc::new(Atomic::new(2084i32));
    let m = oscquery::node::GetSet::new(
        "bar".into(),
        None,
        vec![ParamGetSet::Int(
            ValueBuilder::new(a.clone() as _)
                .with_unit("speed.mph".into())
                .build(),
        )],
        Some(Box::new(move |params, address, time| {
            println!("handler got {:?} {:?} {:?}", params, address, time);
            true
        })),
    );

    std::thread::sleep(std::time::Duration::from_secs(10));
    let parent = res.unwrap();
    let res = root.add_node(m.unwrap().into(), Some(parent.clone()));
    assert!(res.is_ok());

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        root.trigger_path("/foo/bar");
    }
}
