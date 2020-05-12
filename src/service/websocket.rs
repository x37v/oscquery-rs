use std::collections::HashSet;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread::{spawn, JoinHandle};

use serde::{Deserialize, Serialize};

use tungstenite::{accept, error::Error, Message};

use multiqueue::{broadcast_queue, BroadcastSender};
use std::sync::mpsc::TryRecvError;

use crate::root::RootInner;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

//what we set the TCP stream read timeout to
const READ_TIMEOUT: Duration = Duration::from_millis(1);

#[derive(Clone, Debug)]
pub(crate) enum Command {
    Osc(rosc::OscMessage),
    Close,
}

pub struct WSService {
    handle: Option<JoinHandle<()>>,
    cmd_sender: BroadcastSender<Command>,
    local_addr: SocketAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum ClientServerCmd {
    Listen,
    Ignore,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ServerClientCmd {
    PathChanged,
    PathRenamed,
    PathRemoved,
    PathAdded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct WSCommandPacket<T> {
    command: T,
    data: String,
}

impl WSService {
    pub(crate) fn new<A: ToSocketAddrs>(
        root: Arc<RwLock<RootInner>>,
        addr: A,
    ) -> Result<Self, std::io::Error> {
        let server = TcpListener::bind(addr)?;
        let (cmd_sender, recv) = broadcast_queue(32);
        let local_addr = server.local_addr()?;
        let handle = spawn(move || {
            for stream in server.incoming() {
                if stream.is_err() {
                    continue;
                }
                let stream = stream.unwrap();
                stream
                    .set_read_timeout(Some(READ_TIMEOUT))
                    .expect("cannot set read timeout");
                let root = root.clone();
                let msg_recv = recv.clone();
                spawn(move || {
                    let mut listening: HashSet<String> = HashSet::new();
                    if let Ok(mut websocket) = accept(stream) {
                        loop {
                            match msg_recv.try_recv() {
                                Ok(Command::Close) => {
                                    return;
                                }
                                Ok(Command::Osc(m)) => {
                                    //relay osc messages if the remote client has subscribed
                                    if listening.contains(&m.addr) {
                                        if let Ok(buf) =
                                            rosc::encoder::encode(&rosc::OscPacket::Message(m))
                                        {
                                            if websocket
                                                .write_message(Message::Binary(buf))
                                                .is_err()
                                            {
                                                println!("error wrigin osc message");
                                                return;
                                            }
                                        }
                                    }
                                }
                                Err(TryRecvError::Empty) => (),
                                Err(TryRecvError::Disconnected) => return,
                            };
                            match websocket.read_message() {
                                Ok(msg) => {
                                    match msg {
                                        //binary messages are OSC packets
                                        Message::Binary(v) => {
                                            if let Ok(packet) = rosc::decoder::decode(&v) {
                                                if let Ok(root) = root.read() {
                                                    root.handle_osc_packet(&packet, None, None);
                                                }
                                            }
                                        }
                                        Message::Text(s) => {
                                            if let Ok(cmd) =
                                                serde_json::from_str::<
                                                    WSCommandPacket<ClientServerCmd>,
                                                >(&s)
                                            {
                                                match cmd.command {
                                                    ClientServerCmd::Listen => {
                                                        let _ = listening.insert(cmd.data);
                                                    }
                                                    ClientServerCmd::Ignore => {
                                                        let _ = listening.remove(&cmd.data);
                                                    }
                                                }
                                            };
                                        }
                                        Message::Close(..) => return,
                                        Message::Ping(d) => {
                                            //TODO if err, return?
                                            let _ = websocket.write_message(Message::Pong(d));
                                        }
                                        Message::Pong(..) => println!("pong"),
                                    };
                                }
                                Err(Error::ConnectionClosed) | Err(Error::AlreadyClosed) => {
                                    return;
                                }
                                Err(..) => (), //TODO
                            }
                        }
                    }
                });
            }
        });
        Ok(Self {
            handle: Some(handle),
            local_addr,
            cmd_sender,
        })
    }

    /// Returns the `SocketAddr` that the service bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

impl Drop for WSService {
    fn drop(&mut self) {
        /*
         * XXX how to kill the outter thread??
        if self.cmd_sender.try_send(Command::Close).is_ok() {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
        */
    }
}
