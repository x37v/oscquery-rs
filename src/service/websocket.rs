use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::thread::{spawn, JoinHandle};

use serde::{Deserialize, Serialize};

use tungstenite::{accept, error::Error, Message};

use multiqueue::{broadcast_queue, BroadcastSender};
use std::sync::mpsc::{TryRecvError, TrySendError};

use crate::root::{NamespaceChange, RootInner};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

//what we set the TCP stream read timeout to
const READ_TIMEOUT: Duration = Duration::from_millis(1);

#[derive(Clone, Debug)]
pub(crate) enum Command {
    Osc(rosc::OscMessage),
    NamespaceChange(NamespaceChange),
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
        let ns_change_recv = root
            .write()
            .expect("cannot write lock root")
            .ns_change_recv();
        if ns_change_recv.is_none() {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "couldn't get namespace change from root",
            ));
        }
        let ns_change_recv = ns_change_recv.unwrap();
        //XXX how do we set non blocking so we can ditch on drop?
        server
            .set_nonblocking(true)
            .expect("cannot set to non blocking");
        let (cmd_sender, recv) = broadcast_queue(32);
        let local_addr = server.local_addr()?;
        let handle = spawn(move || {
            loop {
                if let Ok(ns_change) = ns_change_recv.try_recv() {
                    //XXX
                }
                let (stream, _addr) = match server.accept() {
                    Ok(s) => s,
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => continue,
                        e @ _ => {
                            println!("tcp accept error {:?}", e);
                            return;
                        }
                    },
                };
                stream
                    .set_read_timeout(Some(READ_TIMEOUT))
                    .expect("cannot set read timeout");
                let root = root.clone();
                let msg_recv = recv.clone();
                spawn(move || {
                    let mut listening: HashSet<String> = HashSet::new();
                    if let Ok(mut websocket) = accept(stream) {
                        loop {
                            //write any commands
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
                                                println!("error writing osc message");
                                            }
                                        }
                                    }
                                }
                                Ok(Command::NamespaceChange(ns)) => {
                                    let s = serde_json::to_string(&match ns {
                                        NamespaceChange::PathAdded(p) => WSCommandPacket {
                                            command: ServerClientCmd::PathAdded,
                                            data: p.clone(),
                                        },
                                        NamespaceChange::PathRemoved(p) => WSCommandPacket {
                                            command: ServerClientCmd::PathRemoved,
                                            data: p.clone(),
                                        },
                                    });
                                    if let Ok(s) = s {
                                        if websocket.write_message(Message::Text(s)).is_err() {
                                            println!("error writing ns message");
                                        }
                                    }
                                }
                                Err(TryRecvError::Empty) => (),
                                Err(TryRecvError::Disconnected) => return,
                            };

                            //react to any incoming
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
                                        Message::Pong(..) => (),
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

    pub fn send(&self, msg: rosc::OscMessage) -> Result<(), TrySendError<rosc::OscMessage>> {
        match self.cmd_sender.try_send(Command::Osc(msg)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(cmd)) => {
                if let Command::Osc(msg) = cmd {
                    Err(TrySendError::Full(msg))
                } else {
                    panic!("should be Osc");
                }
            }
            Err(TrySendError::Disconnected(cmd)) => {
                if let Command::Osc(msg) = cmd {
                    Err(TrySendError::Disconnected(msg))
                } else {
                    panic!("should be Osc");
                }
            }
        }
    }

    /// Returns the `SocketAddr` that the service bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

impl Drop for WSService {
    fn drop(&mut self) {
        if self.cmd_sender.try_send(Command::Close).is_ok() {
            panic!("will never work, until we figure out outter loop");
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }
}
