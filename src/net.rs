//! LAN link (the stand-in for the original infrared port): UDP beacons for discovery and one
//! TCP connection carrying newline-delimited JSON. All socket work runs on background threads;
//! the UI calls `poll()` each frame.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use socket2::{Domain, Protocol, Socket, Type};

/// Protocol version. Bump it whenever messages change; both sides check it during pairing.
/// 3: both sides take turns choosing actions, and the link stays up between them.
pub const PROTO: u64 = 3;
pub const TCP_PORT: u16 = 41975;
pub const BEACON_PORT: u16 = 41976;
const MAGIC: &str = "bytepet";
const MAX_LINE: usize = 16 * 1024;
/// Longest a send may block the UI thread. Messages are tiny, so a write only waits when the
/// other side has stopped reading altogether (its machine froze or the network dropped).
const WRITE_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq)]
pub enum NetEvent {
    Connected,
    Closed,
    Error(String),
    ScanDone,
    Msg(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub ip: String,
    pub port: u16,
    pub name: String,
    pub id: String,
}

pub struct Link {
    tx: Sender<NetEvent>,
    rx: Receiver<NetEvent>,
    stream: Arc<Mutex<Option<TcpStream>>>,
    alive: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    pub found: Arc<Mutex<Vec<Found>>>,
    pub port: Option<u16>,
    token: String,
}

impl Default for Link {
    fn default() -> Self {
        Self::new()
    }
}

impl Link {
    pub fn new() -> Link {
        let (tx, rx) = channel();
        let token = format!("{:08x}", crate::rng::Pcg32::from_time().next_u32());
        Link {
            tx,
            rx,
            stream: Arc::new(Mutex::new(None)),
            alive: Arc::new(AtomicBool::new(true)),
            connected: Arc::new(AtomicBool::new(false)),
            found: Arc::new(Mutex::new(Vec::new())),
            port: None,
            token,
        }
    }

    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    // ------------------------------------------------------------------ hosting
    /// Listen for one connection and announce ourselves on the LAN. Returns the port.
    pub fn host(&mut self, name: &str) -> Result<u16, String> {
        let mut bound = None;
        for p in TCP_PORT..TCP_PORT + 10 {
            if let Ok(l) = listener(p) {
                bound = Some((l, p));
                break;
            }
        }
        let (listener, port) = bound.ok_or("NO PORT")?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        self.port = Some(port);

        let (alive, connected, stream, tx) =
            (self.alive.clone(), self.connected.clone(), self.stream.clone(), self.tx.clone());
        thread::spawn(move || {
            while alive.load(Ordering::SeqCst) && !connected.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((s, _)) => {
                        attach(s, &stream, &connected, &alive, &tx);
                        return;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(100)),
                    Err(_) => return,
                }
            }
        });

        let (alive, connected) = (self.alive.clone(), self.connected.clone());
        let beacon =
            json!({"m": MAGIC, "proto": PROTO, "name": clip(name, 12), "port": port, "id": self.token}).to_string();
        thread::spawn(move || {
            let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) else { return };
            let _ = sock.set_broadcast(true);
            while alive.load(Ordering::SeqCst) && !connected.load(Ordering::SeqCst) {
                for target in ["255.255.255.255", "127.0.0.1"] {
                    let _ = sock.send_to(beacon.as_bytes(), (target, BEACON_PORT));
                }
                for _ in 0..10 {
                    if !alive.load(Ordering::SeqCst) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        });
        Ok(port)
    }

    // ------------------------------------------------------------------ joining
    /// Listen for host beacons for `secs` seconds; sends `ScanDone` at the end.
    pub fn discover(&self, secs: f64) {
        let (alive, found, tx) = (self.alive.clone(), self.found.clone(), self.tx.clone());
        thread::spawn(move || {
            let sock = match beacon_socket() {
                Ok(s) => s,
                Err(_) => {
                    let _ = tx.send(NetEvent::Error("SCAN FAIL".into()));
                    return;
                }
            };
            let end = Instant::now() + Duration::from_secs_f64(secs);
            let mut buf = [0u8; 2048];
            while alive.load(Ordering::SeqCst) && Instant::now() < end {
                let Ok((n, addr)) = sock.recv_from(&mut buf) else { continue };
                let Ok(v) = serde_json::from_slice::<Value>(&buf[..n]) else { continue };
                if v["m"] != MAGIC {
                    continue;
                }
                let (Some(port), Some(id)) = (v["port"].as_u64(), v["id"].as_str()) else { continue };
                let f = Found {
                    ip: addr.ip().to_string(),
                    port: port as u16,
                    name: clip(v["name"].as_str().unwrap_or("?"), 12),
                    id: id.to_string(),
                };
                let mut list = found.lock().unwrap();
                // one entry per host; prefer its LAN address over loopback
                if let Some(old) = list.iter_mut().find(|o| o.id == f.id) {
                    if old.ip.starts_with("127.") && !f.ip.starts_with("127.") {
                        *old = f;
                    }
                } else {
                    list.push(f);
                }
            }
            let _ = tx.send(NetEvent::ScanDone);
        });
    }

    pub fn join(&self, ip: &str, port: u16) {
        let (alive, connected, stream, tx) =
            (self.alive.clone(), self.connected.clone(), self.stream.clone(), self.tx.clone());
        let ip = ip.to_string();
        thread::spawn(move || {
            let addr: Option<SocketAddr> =
                ip.parse::<Ipv4Addr>().ok().map(|a| SocketAddr::V4(SocketAddrV4::new(a, port)));
            let Some(addr) = addr else {
                let _ = tx.send(NetEvent::Error("BAD IP".into()));
                return;
            };
            match TcpStream::connect_timeout(&addr, Duration::from_secs(4)) {
                Ok(s) => attach(s, &stream, &connected, &alive, &tx),
                Err(_) => {
                    let _ = tx.send(NetEvent::Error("NO ANSWER".into()));
                }
            }
        });
    }

    // ------------------------------------------------------------------ messages
    pub fn send(&self, v: &Value) {
        let mut line = v.to_string();
        line.push('\n');
        let mut guard = self.stream.lock().unwrap();
        if let Some(s) = guard.as_mut() {
            if s.write_all(line.as_bytes()).is_err() {
                // timed out or broken: hang up, so later sends don't wait all over again
                if let Some(s) = guard.take() {
                    let _ = s.shutdown(std::net::Shutdown::Both);
                }
                let _ = self.tx.send(NetEvent::Closed);
            }
        }
    }

    pub fn poll(&self) -> Vec<NetEvent> {
        self.rx.try_iter().collect()
    }

    pub fn close(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        if let Some(s) = self.stream.lock().unwrap().take() {
            let _ = s.shutdown(std::net::Shutdown::Both);
        }
        self.connected.store(false, Ordering::SeqCst);
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.close();
    }
}

/// A listening socket that can be reopened right after a previous link (no TIME_WAIT wait).
fn listener(port: u16) -> std::io::Result<TcpListener> {
    let s = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    #[cfg(unix)]
    s.set_reuse_address(true)?;
    s.bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).into())?;
    s.listen(1)?;
    Ok(s.into())
}

fn beacon_socket() -> std::io::Result<UdpSocket> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_reuse_address(true)?;
    #[cfg(all(unix, not(target_os = "solaris")))]
    {
        let _ = s.set_reuse_port(true);
    }
    s.bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT)).into())?;
    let u: UdpSocket = s.into();
    u.set_read_timeout(Some(Duration::from_millis(300)))?;
    Ok(u)
}

fn attach(
    s: TcpStream,
    stream: &Arc<Mutex<Option<TcpStream>>>,
    connected: &Arc<AtomicBool>,
    alive: &Arc<AtomicBool>,
    tx: &Sender<NetEvent>,
) {
    let _ = s.set_nonblocking(false);
    let _ = s.set_nodelay(true);
    // `Link::send` writes on the UI thread: a friend that stops reading mustn't freeze this pet
    let _ = s.set_write_timeout(Some(WRITE_TIMEOUT));
    let Ok(reader) = s.try_clone() else {
        let _ = tx.send(NetEvent::Error("LINK FAIL".into()));
        return;
    };
    *stream.lock().unwrap() = Some(s);
    connected.store(true, Ordering::SeqCst);
    let _ = tx.send(NetEvent::Connected);
    let (alive, connected, tx) = (alive.clone(), connected.clone(), tx.clone());
    thread::spawn(move || read_loop(reader, alive, connected, tx));
}

fn read_loop(mut s: TcpStream, alive: Arc<AtomicBool>, connected: Arc<AtomicBool>, tx: Sender<NetEvent>) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = match s.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_LINE {
            break; // a peer sending garbage: drop the connection
        }
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=pos).collect();
            if let Ok(v) = serde_json::from_slice::<Value>(&line) {
                let ok = v.get("t").and_then(|t| t.as_str()).map(|t| !t.is_empty()).unwrap_or(false);
                if ok && tx.send(NetEvent::Msg(v)).is_err() {
                    return;
                }
            }
        }
    }
    connected.store(false, Ordering::SeqCst);
    if alive.load(Ordering::SeqCst) {
        let _ = tx.send(NetEvent::Closed);
    }
}

/// This machine's LAN address (no packets are sent).
pub fn local_ip() -> String {
    UdpSocket::bind(("0.0.0.0", 0))
        .and_then(|s| {
            s.connect(("10.255.255.255", 1))?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".into())
}

/// Upper-case, printable, at most `n` characters: names from the network are untrusted.
pub fn clip(s: &str, n: usize) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ').take(n).collect::<String>().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for(link: &Link, pred: impl Fn(&NetEvent) -> bool, secs: f64) -> Option<NetEvent> {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        while Instant::now() < end {
            for e in link.poll() {
                if pred(&e) {
                    return Some(e);
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        None
    }

    #[test]
    fn loopback_messages_and_discovery() {
        let mut host = Link::new();
        let port = host.host("ann").unwrap();
        let guest = Link::new();
        guest.discover(2.5);
        assert!(wait_for(&guest, |e| *e == NetEvent::ScanDone, 5.0).is_some());
        let found = guest.found.lock().unwrap().clone();
        assert!(found.iter().any(|f| f.port == port && f.name == "ANN"), "{found:?}");
        guest.join("127.0.0.1", port);
        assert!(wait_for(&host, |e| *e == NetEvent::Connected, 5.0).is_some());
        assert!(wait_for(&guest, |e| *e == NetEvent::Connected, 5.0).is_some());
        guest.send(&json!({"t": "hello", "n": 1}));
        let got = wait_for(&host, |e| matches!(e, NetEvent::Msg(_)), 5.0).unwrap();
        assert_eq!(got, NetEvent::Msg(json!({"t": "hello", "n": 1})));
        host.close();
        assert!(wait_for(&guest, |e| *e == NetEvent::Closed, 5.0).is_some());
    }

    #[test]
    fn a_peer_that_stops_reading_cannot_freeze_the_sender() {
        let mut host = Link::new();
        let port = host.host("ann").unwrap();
        // connects, then never reads a byte
        let _frozen = TcpStream::connect(("127.0.0.1", port)).unwrap();
        assert!(wait_for(&host, |e| *e == NetEvent::Connected, 5.0).is_some());
        let big = json!({"t": "x", "pad": "x".repeat(64 * 1024)});
        let mut closed = false;
        // far more than any socket buffer holds
        for _ in 0..4096 {
            let t = Instant::now();
            host.send(&big);
            assert!(t.elapsed() < WRITE_TIMEOUT * 3, "a send blocked for {:?}", t.elapsed());
            if host.poll().contains(&NetEvent::Closed) {
                closed = true;
                break;
            }
        }
        assert!(closed, "the link never gave up on the frozen peer");
        // and once given up on, sending is instant
        let t = Instant::now();
        host.send(&big);
        assert!(t.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn names_are_sanitised() {
        assert_eq!(clip("bob<script>\n", 8), "BOBSCRIP");
    }
}
