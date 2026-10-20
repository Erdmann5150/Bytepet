//! One Bytepet per save file. The first copy listens on a loopback port derived from the
//! save path; a second launch with the same save asks it to show itself and exits instead
//! of running a second simulation against the same file. Different saves (`--save a.json`,
//! `--save b.json`) get different ports, so they can run side by side.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

const ACK: &[u8] = b"bytepet-ok!\n";
const BASE_PORT: u16 = 42100;
const PORTS: u32 = 800;

pub struct Server {
    listener: TcpListener,
    hello: [u8; 22],
}

pub enum Claim {
    /// We're the only copy. Pass this to [`serve`].
    First(Server),
    /// Another Bytepet has this save and was asked to show itself.
    Running,
    /// The port is taken by something that isn't Bytepet; carry on without the guard.
    Unknown,
}

/// The canonical form of a save path, even if the file doesn't exist yet.
fn canonical(save: &Path) -> PathBuf {
    if let Ok(p) = std::fs::canonicalize(save) {
        return p;
    }
    match (save.parent(), save.file_name()) {
        (Some(dir), Some(name)) => {
            let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
            std::fs::canonicalize(dir).map(|d| d.join(name)).unwrap_or_else(|_| save.to_path_buf())
        }
        _ => save.to_path_buf(),
    }
}

/// FNV-1a of the canonical path: stable across runs (std's hasher is randomly seeded).
fn key(save: &Path) -> u32 {
    let mut text = canonical(save).to_string_lossy().into_owned();
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        text = text.to_lowercase(); // case-insensitive file systems
    }
    let mut h: u32 = 0x811c_9dc5;
    for b in text.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

pub fn port_for(save: &Path) -> u16 {
    BASE_PORT + (key(save) % PORTS) as u16
}

/// The greeting names the save, so two saves that happen to share a port don't mix up.
fn hello(key: u32) -> [u8; 22] {
    let mut out = [0u8; 22];
    out.copy_from_slice(format!("bytepet-show {key:08x}\n").as_bytes());
    out
}

pub fn claim(save: &Path) -> Claim {
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port_for(save));
    if let Ok(l) = TcpListener::bind(addr) {
        return Claim::First(Server { listener: l, hello: hello(key(save)) });
    }
    let Ok(mut s) = TcpStream::connect_timeout(&SocketAddr::V4(addr), Duration::from_millis(500)) else {
        return Claim::Unknown;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = s.set_write_timeout(Some(Duration::from_millis(500)));
    let mut reply = [0u8; ACK.len()];
    if s.write_all(&hello(key(save))).is_ok() && s.read_exact(&mut reply).is_ok() && reply == ACK {
        Claim::Running
    } else {
        Claim::Unknown
    }
}

/// Answer later launches on a background thread, calling `on_show` for each.
pub fn serve(server: Server, on_show: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        for stream in server.listener.incoming() {
            let Ok(mut s) = stream else { continue };
            let _ = s.set_read_timeout(Some(Duration::from_millis(1000)));
            let mut buf = [0u8; 22];
            if s.read_exact(&mut buf).is_ok() && buf == server.hello {
                let _ = s.write_all(ACK);
                on_show();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn a_second_launch_finds_the_first() {
        let dir = std::env::temp_dir().join(format!("bytepet-instance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("save.json");
        let Claim::First(l) = claim(&save) else { panic!("first claim failed (port in use?)") };
        let shown = Arc::new(AtomicUsize::new(0));
        let s2 = shown.clone();
        serve(l, move || {
            s2.fetch_add(1, Ordering::SeqCst);
        });
        assert!(matches!(claim(&save), Claim::Running));
        // the same file named differently is still the same save
        assert!(matches!(claim(&dir.join(".").join("save.json")), Claim::Running));
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(shown.load(Ordering::SeqCst), 2);
        // a different save is independent
        assert_ne!(port_for(&save), port_for(&dir.join("other.json")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
