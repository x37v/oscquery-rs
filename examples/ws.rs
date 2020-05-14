use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use std::thread::spawn;
use tungstenite::accept;

fn main() {
    let server =
        TcpListener::bind(SocketAddr::from_str("127.0.0.1:3002").unwrap()).expect("couldn't bind");
    server
        .set_nonblocking(true)
        .expect("cannot set to non blocking");

    let handle = spawn(move || loop {
        let stream = server.accept();
        let (stream, _addr) = match stream {
            Ok(s) => s,
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => continue,
                e @ _ => {
                    println!("tcp accept error {:?}", e);
                    return;
                }
            },
        };

        println!("spawn websocket");
        spawn(move || {
            if let Ok(mut websocket) = accept(stream) {
                loop {
                    if let Ok(msg) = websocket.read_message() {
                        println!("{:?}", msg);
                    }
                }
            }
        });
    });
    let _ = handle.join();
}
