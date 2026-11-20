//! Background music: plays your own songs from theme\music (mp3, ogg, flac, wav) on the
//! dashboard, like the 360's custom soundtracks. Fades out when a game starts and comes back
//! afterwards. Runs on its own thread.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

pub enum Cmd {
    /// Play (true) or pause with a fade (false).
    Active(bool),
    /// 0.0 - 1.0
    Volume(f32),
    Next,
    Rescan,
}

pub enum Msg {
    NowPlaying(String),
}

pub fn music_dir() -> PathBuf {
    crate::theme_dir().join("music")
}

pub fn songs() -> Vec<PathBuf> {
    let _ = std::fs::create_dir_all(music_dir());
    let mut v: Vec<PathBuf> = walk(&music_dir(), 0);
    v.sort();
    v
}

fn walk(d: &std::path::Path, depth: u32) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(d) else { return vec![] };
    let mut out = vec![];
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && depth < 3 {
            out.extend(walk(&p, depth + 1));
        } else if p.extension().map(|x| matches!(x.to_string_lossy().to_lowercase().as_str(), "mp3" | "ogg" | "flac" | "wav")).unwrap_or(false) {
            out.push(p);
        }
    }
    out
}

pub fn spawn(rx: Receiver<Cmd>, tx: Sender<Msg>, volume: f32) {
    std::thread::spawn(move || {
        // The audio device can be missing (no speakers, remote desktop): then just stay quiet.
        let Ok((_stream, handle)) = rodio::OutputStream::try_default() else { return };
        let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
        let mut list = songs();
        let mut order: Vec<usize> = shuffled(list.len());
        let mut pos = 0usize;
        let mut active = false;
        let mut target_vol = volume;
        let mut level = 0.0f32; // fade level 0..1
        let mut last = Instant::now();
        let start_next = |sink: &rodio::Sink, list: &Vec<PathBuf>, order: &mut Vec<usize>, pos: &mut usize| -> Option<String> {
            for _ in 0..list.len() {
                if *pos >= order.len() {
                    *order = shuffled(list.len());
                    *pos = 0;
                }
                let p = list.get(order[*pos])?.clone();
                *pos += 1;
                let Ok(f) = std::fs::File::open(&p) else { continue };
                let Ok(src) = rodio::Decoder::new(std::io::BufReader::new(f)) else { continue };
                sink.clear();
                sink.append(src);
                return Some(p.file_stem()?.to_string_lossy().replace('_', " "));
            }
            None
        };
        loop {
            while let Ok(c) = rx.try_recv() {
                match c {
                    Cmd::Active(a) => active = a,
                    Cmd::Volume(v) => target_vol = v.clamp(0.0, 1.0),
                    Cmd::Next => {
                        if let Some(n) = start_next(&sink, &list, &mut order, &mut pos) {
                            sink.play();
                            let _ = tx.send(Msg::NowPlaying(n));
                        }
                    }
                    Cmd::Rescan => {
                        list = songs();
                        order = shuffled(list.len());
                        pos = 0;
                        sink.clear();
                    }
                }
            }
            let dt = last.elapsed().as_secs_f32();
            last = Instant::now();
            // Fade in ~0.8 s, out ~0.5 s.
            level = if active { (level + dt / 0.8).min(1.0) } else { (level - dt / 0.5).max(0.0) };
            sink.set_volume(level * level * target_vol);
            if active && !list.is_empty() && target_vol > 0.0 {
                if sink.empty() {
                    if let Some(n) = start_next(&sink, &list, &mut order, &mut pos) {
                        let _ = tx.send(Msg::NowPlaying(n));
                    }
                }
                if sink.is_paused() {
                    sink.play();
                }
            } else if level <= 0.0 && !sink.is_paused() {
                sink.pause();
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    });
}

fn shuffled(n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
    for i in (1..v.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        v.swap(i, (seed % (i as u64 + 1)) as usize);
    }
    v
}
