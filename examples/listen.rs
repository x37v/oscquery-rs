use tungstenite::connect;
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

    loop {
        let v = socket.read_message();
        println!("{:?}", v);
    }
}
