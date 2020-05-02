use ::atomic::Atomic;
use oscquery::param::*;
use oscquery::root::Root;
use oscquery::service::ServiceHandle;
use oscquery::value::*;
use std::sync::Arc;

fn main() {
    let root = Arc::new(Root::new());
    let c = oscquery::node::Container::new("foo".into(), Some("description of foo".into()));
    assert!(c.is_ok());
    let res = root.add_node(c.unwrap().into(), None);
    assert!(res.is_ok());

    let a = Arc::new(Atomic::new(2084i32));
    let m = oscquery::node::Get::new(
        "bar".into(),
        None,
        vec![ParamGet::Int(ValueBuilder::new(a.clone() as _).build())],
    );

    let res = root.add_node(m.unwrap().into(), Some(res.unwrap()));
    assert!(res.is_ok());
    let _handle = ServiceHandle::new(root, 3000);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
