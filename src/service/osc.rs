use crate::node::OscRender;
use crate::root::{NodeHandle, NodeWrapper, RootInner};

use rosc::{OscMessage, OscPacket};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{sync_channel, SyncSender, TryRecvError};
use std::sync::Arc;
use std::sync::RwLock;
use std::thread::JoinHandle;
use std::time::Duration;

//TODO: what we set the TCP stream read timeout to?
const READ_TIMEOUT: Duration = Duration::from_millis(1);
const CHANNEL_LEN: usize = 1024;

/// Manage a thread that reads and writes OSC to/from a socket and updates a values in an OSCQuery tree.
///
/// Drop to stop the service.
/// *NOTE* this will block until the service thread completes.

pub struct OscService {
    root: Arc<RwLock<RootInner>>,
    handle: Option<JoinHandle<()>>,
    cmd_sender: SyncSender<Command>,
    local_addr: SocketAddr,
    send_addrs: RwLock<HashSet<SocketAddr>>,
}

enum Command {
    Send(Vec<u8>, SocketAddr),
    End,
}

impl OscService {
    /// Create and start an OscService
    pub(crate) fn new<A: ToSocketAddrs>(
        root: Arc<RwLock<RootInner>>,
        addr: A,
    ) -> Result<Self, std::io::Error> {
        let sock = UdpSocket::bind(addr)?;
        let local_addr = sock.local_addr()?;
        let (cmd_sender, cmd_recv) = sync_channel(CHANNEL_LEN);

        //timeout reads so we can check our cmd queue
        sock.set_read_timeout(Some(READ_TIMEOUT))?;

        let r = root.clone();
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];
            loop {
                match cmd_recv.try_recv() {
                    Ok(Command::End) => return,
                    Ok(Command::Send(buf, to_addr)) => {
                        //XXX indicate error?
                        let _ = sock.send_to(&buf, to_addr);
                    }
                    Err(TryRecvError::Disconnected) => {
                        return;
                    }
                    Err(TryRecvError::Empty) => (),
                }
                match sock.recv_from(&mut buf) {
                    Ok((size, addr)) => {
                        if size > 0 {
                            let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                            if let Ok(root) = root.read() {
                                root.handle_osc_packet(&packet, Some(addr), None);
                            }
                        }
                    }
                    Err(e) => match e.kind() {
                        //timeout
                        //https://doc.rust-lang.org/std/net/struct.UdpSocket.html#method.set_read_timeout
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => (),
                        _ => {
                            eprintln!("Error receiving from socket: {}", e);
                            break;
                        }
                    },
                };
            }
        });
        Ok(Self {
            root: r,
            handle: Some(handle),
            cmd_sender,
            local_addr,
            send_addrs: RwLock::new(HashSet::new()),
        })
    }

    fn send(&self, buf: &Vec<u8>) {
        if let Ok(addrs) = self.send_addrs.read() {
            for addr in &*addrs {
                if let Err(_) = self
                    .cmd_sender
                    .send(Command::Send(buf.clone(), addr.clone()))
                {
                    eprintln!("error sending to {}", addr);
                }
            }
        }
    }

    fn render_and_send(&self, node: &NodeWrapper) -> Option<OscMessage> {
        let mut args = Vec::new();
        node.node.osc_render(&mut args);
        let addr = node.full_path.clone();
        let msg = OscMessage {
            addr: addr.clone(),
            args,
        };
        let buf = rosc::encoder::encode(&OscPacket::Message(msg.clone()));
        match buf {
            Ok(buf) => {
                self.send(&buf);
                Some(msg)
            }
            Err(..) => {
                eprintln!("error encoding");
                None
            }
        }
    }

    /// Trigger a OSC send for the node at the given handle, if it is valid.
    /// returns the address and renered buffer that was sent, if any
    pub fn trigger(&self, handle: NodeHandle) -> Option<OscMessage> {
        if let Ok(root) = self.root.read() {
            root.with_node_at_handle(&handle, |node| {
                if let Some(node) = node {
                    self.render_and_send(node)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    /// Trigger an OSC send for the node at the given path, if it is valid.
    /// returns the address and renered buffer that was sent, if any
    pub fn trigger_path(&self, path: &str) -> Option<OscMessage> {
        if let Ok(root) = self.root.read() {
            root.with_node_at_path(path, |node| {
                if let Some(node) = node {
                    self.render_and_send(node)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    /// Add an address to send all outgoing OSC messages
    ///
    /// *NOTE* uses a HashSet internally so adding the same address more than once is okay.
    /// This method locks.
    pub fn add_send_addr(&self, addr: SocketAddr) {
        self.send_addrs
            .write()
            .expect("failed to get write lock")
            .insert(addr);
    }

    /// Returns the `SocketAddr` that the service bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

impl Drop for OscService {
    fn drop(&mut self) {
        if self.cmd_sender.send(Command::End).is_ok() {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }
}
