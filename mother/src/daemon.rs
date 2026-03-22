use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use anyhow::Result;

use crate::protocol::{Envelope, PROTOCOL_VERSION};

pub fn listen(socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _ = handle_client(&mut stream);
            }
            Err(_) => continue,
        }
    }
    Ok(())
}

pub fn serve_one(socket_path: &Path) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }
    let listener = UnixListener::bind(socket_path)?;
    if let Ok((mut stream, _)) = listener.accept() {
        let _ = handle_client(&mut stream);
    }
    Ok(())
}

fn handle_client(stream: &mut UnixStream) -> Result<()> {
    let mut line = String::new();
    {
        let mut reader = BufReader::new(stream.try_clone()?);
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
    }

    let parsed: Result<Envelope, _> = serde_json::from_str(&line);
    let response = match parsed {
        Ok(request) if request.v == PROTOCOL_VERSION => Envelope {
            v: PROTOCOL_VERSION,
            action: None,
            payload: None,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        },
        _ => Envelope::unsupported_version(),
    };

    let serialized = serde_json::to_string(&response)?;
    stream.write_all(serialized.as_bytes())?;
    stream.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::thread;

    #[test]
    fn daemon_returns_unsupported_version_for_bad_envelope() {
        let tmp = tempfile::tempdir().unwrap();
        let socket = tmp.path().join("mother.sock");

        let socket_for_thread = socket.clone();
        let handle = thread::spawn(move || serve_one(&socket_for_thread));

        for _ in 0..30 {
            if socket.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let mut stream = UnixStream::connect(&socket).unwrap();
        stream
            .write_all(b"{\"v\":2,\"action\":\"connect\"}\n")
            .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("unsupported protocol version"));
        handle.join().unwrap().unwrap();
    }
}
