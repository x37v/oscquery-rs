use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::thread::{spawn, JoinHandle};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::stream::{StreamExt, TryStreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;
use tungstenite::protocol::Message;

use serde::{Deserialize, Serialize};

use std::sync::mpsc::{channel, sync_channel, SyncSender, TryRecvError};

use crate::root::{NamespaceChange, RootInner};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;

//what we set the TCP stream read timeout to
const CHANNEL_LEN: usize = 1024;

#[derive(Clone, Debug)]
enum Command {
    Osc(rosc::OscMessage),
    Close,
}

/// The websocket service for OSCQuery.
pub struct WSService {
    handle: Option<JoinHandle<()>>,
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

/*
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
*/

#[derive(Clone, Debug)]
enum HandleCommand {
    Close,
    Osc(rosc::OscMessage),
    NamespaceChange(NamespaceChange),
}

type Broadcast = Arc<Mutex<HashMap<SocketAddr, UnboundedSender<HandleCommand>>>>;

async fn handle_connection(
    stream: TcpStream,
    mut rx: UnboundedReceiver<HandleCommand>,
    root: Arc<RwLock<RootInner>>,
) -> Result<(), tungstenite::error::Error> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut outgoing, incoming) = ws.split();

    let listening: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let close = Arc::new(AtomicBool::new(false));

    let ilistening = listening.clone();
    let iclose = close.clone();
    let incoming = incoming.try_for_each(|msg| {
        if iclose.load(Ordering::Relaxed) {
            return futures::future::err(tungstenite::error::Error::ConnectionClosed);
        }
        match msg {
            Message::Ping(d) => {
                /*
                let _ = Runtime::new()
                    .expect("couldn't get send runtime")
                    .block_on(outgoing.send(Message::Pong(d)));
                */
            }
            Message::Pong(..) => (),
            Message::Close(..) => {
                iclose.store(true, Ordering::Relaxed);
                return futures::future::err(tungstenite::error::Error::ConnectionClosed);
            }
            Message::Text(v) => {
                if let Ok(cmd) = serde_json::from_str::<WSCommandPacket<ClientServerCmd>>(&v) {
                    match cmd.command {
                        ClientServerCmd::Listen => {
                            let _ = ilistening.lock().unwrap().insert(cmd.data);
                        }
                        ClientServerCmd::Ignore => {
                            let _ = ilistening.lock().unwrap().remove(&cmd.data);
                        }
                    }
                };
            }
            Message::Binary(v) => {
                if let Ok(packet) = rosc::decoder::decode(&v) {
                    if let Ok(root) = root.read() {
                        root.handle_osc_packet(&packet, None, None);
                    }
                }
            }
        }
        futures::future::ok(())
    });

    let cmds = tokio::spawn(async move {
        while let Ok(cmd) = rx.try_next() {
            if close.load(Ordering::Relaxed) {
                return;
            }
            if let Some(cmd) = cmd {
                match cmd {
                    HandleCommand::Close => {
                        close.store(true, Ordering::Relaxed);
                        return;
                    }
                    HandleCommand::Osc(m) => {
                        //relay osc messages if the remote client has subscribed
                        let send = if let Ok(l) = listening.lock() {
                            l.contains(&m.addr)
                        } else {
                            false
                        };
                        if send {
                            if let Ok(buf) =
                                rosc::encoder::encode(&rosc::OscPacket::Message(m.clone()))
                            {
                                if outgoing.send(Message::Binary(buf)).await.is_err() {
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
                            if outgoing.send(Message::Text(s)).await.is_err() {
                                eprintln!("error writing ns message");
                            }
                        }
                    }
                }
            }
        }
    });

    futures::future::select(incoming, cmds).await;
    Ok(())
}

impl WSService {
    pub(crate) fn new<A: tokio::net::ToSocketAddrs>(
        root: Arc<RwLock<RootInner>>,
        _addr: A,
    ) -> Result<Self, std::io::Error> {
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

        let (cmd_send, cmd_recv) = sync_channel(CHANNEL_LEN);

        let addr = SocketAddr::V4(SocketAddrV4::from_str("127.0.0.1:44444").unwrap()); //XXX TODO
        let local_addr = addr.clone();

        let handle = spawn(move || {
            let mut rt = tokio::runtime::Builder::new()
                .basic_scheduler()
                .threaded_scheduler()
                .enable_all()
                .build()
                .expect("could not create runtime");
            rt.block_on(async move {
                let broadcast: Broadcast = Arc::new(Mutex::new(HashMap::new()));
                println!("outter block");

                let bc = broadcast.clone();
                let ns = tokio::spawn(async move {
                    let broadcast = bc;
                    //read from channel and write
                    loop {
                        match ns_change_recv.recv() {
                            Ok(c) => {
                                println!("ns chnage {:?}", c);
                                let c = HandleCommand::NamespaceChange(c);
                                if let Ok(bl) = broadcast.lock() {
                                    for b in bl.values() {
                                        let _ = b.unbounded_send(c.clone());
                                        //TODO if error, remove
                                    }
                                }
                            }
                            Err(e) => {
                                println!("cmd error {:?}", e);
                                break;
                            }
                        }
                    }
                    println!("exit ns loop");
                });
                let spawn = tokio::spawn(async move {
                    let mut listener = TcpListener::bind(addr).await.unwrap();
                    println!("ws addr {:?}", listener.local_addr());
                    loop {
                        println!("loop enter");
                        match listener.accept().await {
                            Ok((stream, addr)) => {
                                println!("accept");
                                let (tx, rx) = unbounded();
                                if let Ok(mut bl) = broadcast.lock() {
                                    bl.insert(addr, tx);
                                } else {
                                    continue;
                                }
                                let r = root.clone();
                                let bc = broadcast.clone();
                                tokio::spawn(async move {
                                    println!("ws spawn");
                                    let _ = handle_connection(stream, rx, r).await;
                                    if let Ok(mut bl) = bc.lock() {
                                        bl.remove(&addr);
                                    }
                                });
                            }
                            Err(e) => {
                                println!("error accept {:?}", e);
                                break;
                            }
                        };
                    }
                    println!("exiting spawn");
                });
                futures::future::select(ns, spawn).await;
            });
        });

        /*
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
                Err(e) => println!("error accepting {:?}", e),
            }
        });
        */
        Ok(Self {
            handle: Some(handle),
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
            if let Some(_handle) = self.handle.take() {
                panic!("will never work, until we figure out outter loop");
                //let _ = handle.join();
                //let _ = handles.1.join();
            }
        }
    }
}
