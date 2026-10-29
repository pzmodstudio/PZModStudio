//! Enough of the Source RCON protocol to send a dedicated server one command and read its answer.
//!
//! Project Zomboid's server listens for RCON on `RCONPort` when `RCONPassword` is set, and its
//! `quit` command saves the world and exits. Killing the process instead loses everything since
//! the last autosave, so Stop goes through here first.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const AUTH: i32 = 3;
const EXEC: i32 = 2;
const AUTH_RESPONSE: i32 = 2;

fn packet(id: i32, kind: i32, body: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(14 + body.len());
    let size = (4 + 4 + body.len() + 2) as i32;
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

/// One packet back: (id, type, body).
fn read_packet(stream: &mut TcpStream) -> Result<(i32, i32, String), String> {
    let mut size = [0u8; 4];
    stream.read_exact(&mut size).map_err(|e| e.to_string())?;
    let size = i32::from_le_bytes(size);
    if !(10..=1 << 20).contains(&size) {
        return Err(format!("RCON answered with a packet of {} bytes", size));
    }
    let mut rest = vec![0u8; size as usize];
    stream.read_exact(&mut rest).map_err(|e| e.to_string())?;
    let id = i32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
    let kind = i32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]);
    let body = String::from_utf8_lossy(&rest[8..rest.len().saturating_sub(2)]).to_string();
    Ok((id, kind, body))
}

/// Logs in on `127.0.0.1:port` and sends `command`. Ok once the server has accepted it, with
/// what the server answered (empty when no answer came).
pub fn send(port: u16, password: &str, command: &str) -> Result<String, String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("RCON: {}", e))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    stream.write_all(&packet(1, AUTH, password)).map_err(|e| format!("RCON: {}", e))?;
    // Some servers send an empty response before the auth answer.
    loop {
        let (id, kind, _) = read_packet(&mut stream)?;
        if kind == AUTH_RESPONSE {
            if id == -1 {
                return Err("RCON refused the password.".to_string());
            }
            break;
        }
    }
    stream.write_all(&packet(2, EXEC, command)).map_err(|e| format!("RCON: {}", e))?;
    // The answer may never come for "quit" (the server is going away): sent is enough.
    Ok(read_packet(&mut stream).map(|(_, _, body)| body.trim().to_string()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn logs_in_and_sends_the_command() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let (id, kind, body) = read_packet(&mut s).unwrap();
            assert_eq!((kind, body.as_str()), (AUTH, "secret"));
            s.write_all(&packet(id, AUTH_RESPONSE, "")).unwrap();
            let (id, kind, body) = read_packet(&mut s).unwrap();
            s.write_all(&packet(id, 0, "Quitting")).unwrap();
            (kind, body)
        });
        send(port, "secret", "quit").unwrap();
        assert_eq!(server.join().unwrap(), (EXEC, "quit".to_string()));
    }

    #[test]
    fn a_wrong_password_is_an_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = read_packet(&mut s).unwrap();
            s.write_all(&packet(-1, AUTH_RESPONSE, "")).unwrap();
        });
        assert!(send(port, "wrong", "quit").is_err());
    }
}
