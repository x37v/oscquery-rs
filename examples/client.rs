use tungstenite::{connect, Message};
use url::Url;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("you must provide the websocket address to connect to");
        return;
    }
    let (mut socket, response) = connect(Url::parse(&format!("ws://{}/socket", args[1])).unwrap())
        .expect("couldn't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");

    socket
        .write_message(Message::Text(
            "{\"COMMAND\":\"LISTEN\",\"DATA\":\"/foo/bar\"}".into(),
        ))
        .expect("error writing message");

    loop {
        match socket.read_message() {
            Ok(Message::Close(..)) | Err(..) => break,
            Ok(m) => {
                println!("{:?}", m);
            }
        }
    }
    socket.close(None).unwrap();
}
