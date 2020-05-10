use rosc::{OscMessage, OscPacket, OscType};
use tungstenite::{connect, Message};
use url::Url;

fn main() {
    let (mut socket, response) =
        connect(Url::parse("ws://localhost:3002/socket").unwrap()).expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (ref header, _value) in response.headers() {
        println!("* {}", header);
    }

    socket
        .write_message(Message::Text(
            "{\"COMMAND\":\"LISTEN\",\"DATA\":\"awy\"}".into(),
        ))
        .unwrap();

    let v = vec![OscType::Int(22)];
    let buf = rosc::encoder::encode(&OscPacket::Message(OscMessage {
        addr: "/foo/bar".to_string(),
        args: v,
    }))
    .unwrap();
    socket.write_message(Message::Binary(buf)).unwrap();
    //let msg = socket.read_message().expect("Error reading message");
    //println!("Received: {}", msg);
    // socket.close(None);
}
