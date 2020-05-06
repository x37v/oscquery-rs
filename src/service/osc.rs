use crate::root::{NodeHandle, RootInner};
use rosc::OscPacket;
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::JoinHandle;

/// Manage a thread that reads and writes OSC to/from a socket and updates a OSCQuery tree.
///
/// Drop to stop the service.
/// *NOTE* this will block until the service thread completes.

pub struct OscService {
    handle: Option<JoinHandle<()>>,
    done: Arc<AtomicBool>,
    local_addr: SocketAddr,
}

impl OscService {
    /// Create and start an OscService
    pub(crate) fn new<A: ToSocketAddrs>(
        root: Arc<RwLock<RootInner>>,
        addr: A,
    ) -> Result<Self, std::io::Error> {
        let d = Arc::new(AtomicBool::new(false));
        let done = d.clone();
        let sock = UdpSocket::bind(addr)?;
        let local_addr = sock.local_addr()?;
        //timeout reads so we can check our done status
        sock.set_read_timeout(Some(std::time::Duration::from_millis(1)))?;
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; rosc::decoder::MTU];
            while !done.load(Ordering::Relaxed) {
                match sock.recv_from(&mut buf) {
                    Ok((size, _addr)) => {
                        if size > 0 {
                            let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                            if let Ok(root) = root.read() {
                                root.handle_osc_packet(&packet);
                            }
                        }
                    }
                    Err(e) => match e.kind() {
                        //timeout
                        //https://doc.rust-lang.org/std/net/struct.UdpSocket.html#method.set_read_timeout
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => (),
                        _ => {
                            println!("Error receiving from socket: {}", e);
                            break;
                        }
                    },
                };
            }
        });
        Ok(Self {
            handle: Some(handle),
            done: d,
            local_addr,
        })
    }

    pub fn trigger(&self, handle: NodeHandle) {
        //XXX
    }

    pub fn trigger_path(&self, path: &str) {
        //XXX
    }

    /// Returns the `SocketAddr` that the service bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }
}

impl Drop for OscService {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
