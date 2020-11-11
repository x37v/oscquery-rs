use futures::stream::FuturesUnordered;
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs};
use std::thread::{spawn, JoinHandle};

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::stream::StreamExt;

use tokio::net::{TcpListener, TcpStream};
use tungstenite::protocol::Message;

use serde::{Deserialize, Serialize};

use std::sync::mpsc::{sync_channel, SyncSender, TryRecvError};

use crate::root::{NamespaceChange, RootInner};
use std::sync::Arc;
use std::sync::RwLock;

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

#[derive(Clone, Debug)]
enum HandleCommand {
    Close,
    Osc(rosc::OscMessage),
    NamespaceChange(NamespaceChange),
}

type Broadcast = Arc<tokio::sync::Mutex<HashMap<SocketAddr, UnboundedSender<HandleCommand>>>>;

async fn handle_connection(
    stream: TcpStream,
    mut rx: UnboundedReceiver<HandleCommand>,
    root: Arc<RwLock<RootInner>>,
) -> Result<(), tungstenite::error::Error> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut outgoing, mut incoming) = ws.split();
    let mut tasks = FuturesUnordered::new();
    let close = Arc::new(AtomicBool::new(false));

    let (tx, mut orx) = unbounded();
    let iclose = close.clone();
    tasks.push(tokio::spawn(async move {
        while let Some(msg) = orx.next().await {
            match outgoing.send(msg).await {
                Ok(()) => (),
                Err(tungstenite::error::Error::ConnectionClosed) => {
                    iclose.store(true, Ordering::Relaxed);
                    break;
                }
                Err(e) => {
                    eprintln!("error writing to ws sink {:?}", e);
                    break;
                }
            }
        }
    }));
    let mut outgoing = tx;

    let listening: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    let ilistening = listening.clone();
    let iclose = close.clone();
    let mut out = outgoing.clone();
    let incoming = tokio::spawn(async move {
        while let Some(msg) = incoming.next().await {
            match msg {
                Ok(Message::Ping(d)) => {
                    if let Err(e) = out.send(Message::Pong(d)).await {
                        eprintln!("error writing pong {:?}", e);
                    }
                }
                Ok(Message::Pong(..)) => (),
                Ok(Message::Close(..)) => {
                    iclose.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(Message::Text(v)) => {
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
                Ok(Message::Binary(v)) => {
                    if let Ok(packet) = rosc::decoder::decode(&v) {
                        crate::root::RootInner::handle_osc_packet(&root, &packet, None, None);
                    }
                }
                Err(e) => {
                    eprintln!("error on ws incoming {:?}", e);
                    break;
                }
            };
        }
    });
    tasks.push(incoming);

    let cmds = tokio::spawn(async move {
        loop {
            if close.load(Ordering::Relaxed) {
                break;
            }
            match rx.next().await {
                None => break,
                Some(HandleCommand::Close) => {
                    close.store(true, Ordering::Relaxed);
                    break;
                }
                Some(HandleCommand::Osc(m)) => {
                    //relay osc messages if the remote client has subscribed
                    let send = if let Ok(l) = listening.lock() {
                        l.contains(&m.addr)
                    } else {
                        false
                    };
                    if send {
                        if let Ok(buf) = rosc::encoder::encode(&rosc::OscPacket::Message(m.clone()))
                        {
                            if let Err(e) = outgoing.send(Message::Binary(buf)).await {
                                eprintln!("error writing osc message {:?}", e);
                            }
                        }
                    }
                }
                Some(HandleCommand::NamespaceChange(c)) => {
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
                        if let Err(e) = outgoing.send(Message::Text(s)).await {
                            eprintln!("error writing ns message {:?}", e);
                        }
                    }
                }
            };
        }
    });
    tasks.push(cmds);

    while let Some(_) = tasks.next().await {}
    println!("ws exiting");
    Ok(())
}

impl WSService {
    pub(crate) fn new<A: ToSocketAddrs>(
        root: Arc<RwLock<RootInner>>,
        addr: A,
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

        let listener = std::net::TcpListener::bind(addr)?;
        let local_addr = listener.local_addr()?;

        let handle = spawn(move || {
            let mut rt = tokio::runtime::Builder::new()
                .basic_scheduler()
                .threaded_scheduler()
                .enable_all()
                .build()
                .expect("could not create runtime");
            rt.block_on(async move {
                let broadcast: Broadcast = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

                let bc = broadcast.clone();
                let ns = tokio::spawn(async move {
                    let broadcast = bc;
                    //read from channel and write
                    loop {
                        let ns = ns_change_recv.try_recv();
                        let y = match ns {
                            Ok(c) => {
                                let c = HandleCommand::NamespaceChange(c);
                                for mut b in broadcast.lock().await.values() {
                                    if let Err(e) = b.send(c.clone()).await {
                                        eprintln!(
                                            "error writing HandleCommand::NamespaceChange {:?}",
                                            e
                                        );
                                    }
                                }
                                false
                            }
                            Err(TryRecvError::Empty) => true,
                            Err(e) => {
                                eprintln!("cmd error {:?}", e);
                                return;
                            }
                        };

                        let cmd = cmd_recv.try_recv();
                        let y = match cmd {
                            Ok(Command::Close) => {
                                for mut b in broadcast.lock().await.values() {
                                    if let Err(e) = b.send(HandleCommand::Close).await {
                                        eprintln!("error writing HandleCommand::Close {:?}", e);
                                    }
                                }
                                return;
                            }
                            Ok(Command::Osc(m)) => {
                                let c = HandleCommand::Osc(m);
                                for mut b in broadcast.lock().await.values() {
                                    if let Err(e) = b.send(c.clone()).await {
                                        eprintln!("error writing HandleCommand::Osc {:?}", e);
                                    }
                                }
                                false
                            }
                            Err(TryRecvError::Empty) => true,
                            Err(e) => {
                                eprintln!("cmd error {:?}", e);
                                return;
                            }
                        } && y;
                        if y {
                            tokio::task::yield_now().await;
                            tokio::time::delay_for(tokio::time::Duration::from_millis(1)).await;
                        }
                    }
                });
                let spawn = tokio::spawn(async move {
                    let mut listener = TcpListener::from_std(listener).expect(
                        "failed to convert std::net::TcpListener to tokio::net::TcpListener",
                    );
                    loop {
                        match listener.accept().await {
                            Ok((stream, addr)) => {
                                let (tx, rx) = unbounded();
                                broadcast.lock().await.insert(addr, tx);
                                let r = root.clone();
                                let bc = broadcast.clone();
                                tokio::spawn(async move {
                                    let _ = handle_connection(stream, rx, r).await;
                                    bc.lock().await.remove(&addr);
                                });
                            }
                            Err(e) => {
                                eprintln!("error accept {:?}", e);
                                break;
                            }
                        };
                    }
                });
                futures::future::select(ns, spawn).await;
            });
        });

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
            if let Some(handle) = self.handle.take() {
                if let Err(e) = handle.join() {
                    eprintln!("error joining ws thread {:?}", e);
                }
            }
        }
    }
}
