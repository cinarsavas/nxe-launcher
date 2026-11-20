//! Discord status ("Playing Zelda 64: Recompiled · Nintendo 64") through Discord's local
//! IPC pipe, the same way the official Game SDK does it. Needs the Discord desktop app open
//! and a Discord Application ID (created once at discord.com/developers).

use serde_json::json;
use std::io::{Read, Write};
use std::sync::mpsc::Receiver;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub details: String,
    pub state: String,
    /// Unix seconds.
    pub start: Option<i64>,
}

pub enum Cmd {
    Set(Option<Activity>),
    AppId(String),
}

trait Pipe: Read + Write + Send {}
impl<T: Read + Write + Send> Pipe for T {}

fn connect() -> Option<Box<dyn Pipe>> {
    for i in 0..10 {
        #[cfg(windows)]
        {
            if let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}")) {
                return Some(Box::new(f));
            }
        }
        #[cfg(unix)]
        {
            let base = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"].iter().find_map(|v| std::env::var(v).ok()).unwrap_or_else(|| "/tmp".into());
            if let Ok(s) = std::os::unix::net::UnixStream::connect(format!("{base}/discord-ipc-{i}")) {
                let _ = s.set_read_timeout(Some(Duration::from_secs(3)));
                return Some(Box::new(s));
            }
        }
    }
    None
}

/// Frame: opcode (u32 LE), length (u32 LE), JSON.
pub fn frame(op: u32, body: &serde_json::Value) -> Vec<u8> {
    let b = body.to_string().into_bytes();
    let mut v = Vec::with_capacity(8 + b.len());
    v.extend_from_slice(&op.to_le_bytes());
    v.extend_from_slice(&(b.len() as u32).to_le_bytes());
    v.extend_from_slice(&b);
    v
}

fn read_frame(p: &mut dyn Pipe) -> Option<(u32, serde_json::Value)> {
    let mut h = [0u8; 8];
    p.read_exact(&mut h).ok()?;
    let op = u32::from_le_bytes(h[..4].try_into().ok()?);
    let len = u32::from_le_bytes(h[4..].try_into().ok()?) as usize;
    if len > 1 << 20 {
        return None;
    }
    let mut b = vec![0u8; len];
    p.read_exact(&mut b).ok()?;
    Some((op, serde_json::from_slice(&b).ok()?))
}

pub fn activity_json(a: &Option<Activity>) -> serde_json::Value {
    let act = a.as_ref().map(|a| {
        let mut v = json!({
            "details": a.details,
            "state": a.state,
            "assets": { "large_image": "nxe", "large_text": "NXE Launcher" },
        });
        if let Some(s) = a.start {
            v["timestamps"] = json!({ "start": s });
        }
        v
    });
    json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": act },
        "nonce": format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)),
    })
}

pub fn spawn(rx: Receiver<Cmd>, app_id: String) {
    std::thread::spawn(move || {
        let mut app_id = app_id;
        let mut want: Option<Activity> = None;
        let mut pipe: Option<Box<dyn Pipe>> = None;
        let mut sent: Option<Option<Activity>> = None;
        let mut next_try = std::time::Instant::now();
        loop {
            // Wait for something to do (or retry every few seconds).
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(Cmd::Set(a)) => want = a,
                Ok(Cmd::AppId(id)) => {
                    app_id = id;
                    pipe = None;
                    sent = None;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                Err(_) => {}
            }
            while let Ok(c) = rx.try_recv() {
                match c {
                    Cmd::Set(a) => want = a,
                    Cmd::AppId(id) => {
                        app_id = id;
                        pipe = None;
                        sent = None;
                    }
                }
            }
            if app_id.trim().is_empty() {
                continue;
            }
            if sent.as_ref() == Some(&want) && pipe.is_some() {
                continue;
            }
            if pipe.is_none() {
                if std::time::Instant::now() < next_try {
                    continue;
                }
                next_try = std::time::Instant::now() + Duration::from_secs(15);
                let Some(mut p) = connect() else { continue };
                if p.write_all(&frame(0, &json!({ "v": 1, "client_id": app_id.trim() }))).is_err() {
                    continue;
                }
                // READY (or an error about the app ID).
                match read_frame(p.as_mut()) {
                    Some((1, v)) if v["evt"] == "READY" => pipe = Some(p),
                    _ => continue,
                }
            }
            let ok = pipe.as_mut().map(|p| p.write_all(&frame(1, &activity_json(&want))).is_ok() && read_frame(p.as_mut()).is_some()).unwrap_or(false);
            if ok {
                sent = Some(want.clone());
            } else {
                pipe = None;
                sent = None;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing() {
        let f = frame(0, &json!({"v":1,"client_id":"123"}));
        assert_eq!(u32::from_le_bytes(f[..4].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(f[4..8].try_into().unwrap()) as usize, f.len() - 8);
        let a = activity_json(&Some(Activity { details: "Zelda".into(), state: "Nintendo 64".into(), start: Some(5) }));
        assert_eq!(a["args"]["activity"]["timestamps"]["start"], 5);
        assert!(activity_json(&None)["args"]["activity"].is_null());
    }
}
