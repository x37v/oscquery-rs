use tungstenite::{connect, Message};
use url::Url;

fn main() {
    let (mut socket, response) =
        connect(Url::parse("ws://localhost:5678/socket").unwrap()).expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");

    for (ref header, _value) in response.headers() {
        println!("* {}", header);
    }

    socket
        .write_message(Message::Text(
            "{\"COMMAND\":\"IGNORE\",\"DATA\":\"/foo/bar\"}".into(),
        ))
        .unwrap();
    socket.close(None);
}
