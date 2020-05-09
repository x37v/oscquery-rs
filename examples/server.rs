use ::atomic::Atomic;
use oscquery::param::*;
use oscquery::root::Root;
use oscquery::service::http::ServiceHandle;
use oscquery::value::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

fn main() {
    let root = Arc::new(Root::new(Some("server example".into())));
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

    let res = root.add_node(m.unwrap().into(), Some(res.unwrap()));
    assert!(res.is_ok());

    let mut osc = root.spawn_osc("127.0.0.1:3001").unwrap();
    osc.add_send_addr(SocketAddr::from_str("127.0.0.1:3010").unwrap());

    let _handle = ServiceHandle::new(
        root,
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000),
    );
    std::thread::sleep(std::time::Duration::from_secs(1));
    osc.trigger_path("/foo/bar");
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
