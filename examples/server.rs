use oscquery::root::Root;
use oscquery::service::ServiceHandle;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let root = Arc::new(Root::new());
    let _handle = ServiceHandle::new(root);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
