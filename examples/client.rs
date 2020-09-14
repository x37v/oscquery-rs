use rosc::{OscMessage, OscPacket, OscType};
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

    // add a node 'soda'
    let buf = rosc::encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/foo/add".to_string(),
        args: vec![OscType::String("soda".to_string())],
    }))
    .unwrap();

    socket
        .write_message(Message::Binary(buf))
        .expect("error writing");

    socket
        .write_message(Message::Text(
            "{\"COMMAND\":\"LISTEN\",\"DATA\":\"/foo/bar\"}".into(),
        ))
        .expect("error writing message");

    loop {
        match socket.read_message() {
            Ok(Message::Close(..)) => break,
            Err(e) => {
                println!("error {:?}", e);
                break;
            }
            Ok(m) => {
                println!("{:?}", m);
            }
        }
    }
    socket.close(None).unwrap();
}
