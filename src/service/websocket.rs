use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::thread::{spawn, JoinHandle};

use serde::{Deserialize, Serialize};

use std::sync::mpsc::{channel, sync_channel, SyncSender, TryRecvError};
use tungstenite::{accept, error::Error, Message, WebSocket};

use crate::root::{NamespaceChange, RootInner};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

//what we set the TCP stream read timeout to
const READ_TIMEOUT: Duration = Duration::from_millis(1);
const CHANNEL_LEN: usize = 1024;

#[derive(Clone, Debug)]
enum Command {
    Osc(rosc::OscMessage),
    Close,
}

/// The websocket service for OSCQuery.
pub struct WSService {
    handles: Option<(JoinHandle<()>, JoinHandle<()>)>,
    cmd_sender: SyncSender<Command>,
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
    //PathRenamed,
    PathRemoved,
    PathAdded,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct WSCommandPacket<T> {
    command: T,
    data: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum HandleContinue {
    Yes,
    No,
}

struct WSHandle {
    listening: HashSet<String>,
    ws: WebSocket<TcpStream>,

    root: Arc<RwLock<RootInner>>,
}

impl WSHandle {
    pub fn new(ws: WebSocket<TcpStream>, root: Arc<RwLock<RootInner>>) -> Self {
        Self {
            listening: HashSet::new(),
            ws,
            root,
        }
    }

    pub fn process(&mut self, cmds: &Vec<HandleCommand>) -> HandleContinue {
        //handle incoming commands
        for cmd in cmds {
            match cmd {
                HandleCommand::Osc(m) => {
                    //relay osc messages if the remote client has subscribed
                    if self.listening.contains(&m.addr) {
                        if let Ok(buf) = rosc::encoder::encode(&rosc::OscPacket::Message(m.clone()))
                        {
                            if self.ws.write_message(Message::Binary(buf)).is_err() {
                                eprintln!("error writing osc message");
                            }
                        }
                    }
                }
                HandleCommand::NamespaceChange(c) => {
                    let s = serde_json::to_string(&match c {
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
                        if self.ws.write_message(Message::Text(s)).is_err() {
                            eprintln!("error writing ns message");
                        }
                    }
                }
            }
        }
        //handle read messages
        match self.ws.read_message() {
            Ok(msg) => {
                match msg {
                    //binary messages are OSC packets
                    Message::Binary(v) => {
                        if let Ok(packet) = rosc::decoder::decode(&v) {
                            if let Ok(root) = self.root.read() {
                                root.handle_osc_packet(&packet, None, None);
                            }
                        }
                    }
                    Message::Text(s) => {
                        if let Ok(cmd) =
                            serde_json::from_str::<WSCommandPacket<ClientServerCmd>>(&s)
                        {
                            match cmd.command {
                                ClientServerCmd::Listen => {
                                    let _ = self.listening.insert(cmd.data);
                                }
                                ClientServerCmd::Ignore => {
                                    let _ = self.listening.remove(&cmd.data);
                                }
                            }
                        };
                    }
                    Message::Close(..) => return HandleContinue::No,
                    Message::Ping(d) => {
                        //TODO if err, return?
                        let _ = self.ws.write_message(Message::Pong(d));
                    }
                    Message::Pong(..) => (),
                };
            }
            Err(Error::ConnectionClosed) | Err(Error::AlreadyClosed) => {
                return HandleContinue::No;
            }
            Err(..) => (), //TODO
        }
        HandleContinue::Yes
    }
}

enum HandleCommand {
    Osc(rosc::OscMessage),
    NamespaceChange(NamespaceChange),
}

impl WSService {
    pub(crate) fn new<A: ToSocketAddrs>(
        root: Arc<RwLock<RootInner>>,
        addr: A,
    ) -> Result<Self, std::io::Error> {
        let server = TcpListener::bind(addr)?;

        //get the namespace change channel
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
        //server
        //.set_nonblocking(true)
        //.expect("cannot set to non blocking");
        let (ws_send, ws_recv) = channel();
        let (cmd_send, cmd_recv) = sync_channel(CHANNEL_LEN);
        let local_addr = server.local_addr()?;

        //loop over websockets and execute them until complete
        let ws_handle = spawn(move || {
            let mut websockets: Vec<WSHandle> = Vec::new();
            let mut cmds: Vec<HandleCommand> = Vec::new();
            loop {
                while let Ok(s) = ws_recv.try_recv() {
                    websockets.push(s);
                }
                while let Ok(c) = ns_change_recv.try_recv() {
                    cmds.push(HandleCommand::NamespaceChange(c));
                }
                loop {
                    match cmd_recv.try_recv() {
                        Ok(Command::Close) => {
                            return;
                        }
                        Ok(Command::Osc(m)) => {
                            cmds.push(HandleCommand::Osc(m));
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => return,
                    };
                }
                //run the websocket process methods, filter out those that shouldn't keep going
                let mut next = Vec::new();
                for mut ws in websockets {
                    if ws.process(&cmds) == HandleContinue::Yes {
                        next.push(ws);
                    }
                }
                websockets = next;
                cmds.clear();
            }
        });

        let spawn_handle = spawn(move || loop {
            let (stream, _addr) = match server.accept() {
                Ok(s) => s,
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::TimedOut => continue,
                    e @ _ => {
                        eprintln!("tcp accept error {:?}", e);
                        return;
                    }
                },
            };
            //println!("spawning");
            stream
                .set_read_timeout(Some(READ_TIMEOUT))
                .expect("cannot set read timeout");
            match accept(stream) {
                Ok(websocket) => {
                    if ws_send
                        .send(WSHandle::new(websocket, root.clone()))
                        .is_err()
                    {
                        return; //should only happen if the other thread ended
                    }
                }
                Err(e) => eprintln!("error accepting {:?}", e),
            }
        });
        Ok(Self {
            handles: Some((spawn_handle, ws_handle)),
            local_addr,
            cmd_sender: cmd_send,
        })
    }

    pub fn send(&self, msg: rosc::OscMessage) {
        let _ = self.cmd_sender.send(Command::Osc(msg));
    }

    /// Returns the `SocketAddr` that the service bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

impl Drop for WSService {
    fn drop(&mut self) {
        if self.cmd_sender.send(Command::Close).is_ok() {
            if let Some(_handles) = self.handles.take() {
                panic!("will never work, until we figure out outter loop");
                //let _ = handles.0.join();
                //let _ = handles.1.join();
            }
        }
    }
}
