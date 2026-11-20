//! Cover art: automatic download in the background and candidate lists for the
//! "choose a cover" picker.
//!
//! Sources
//! - libretro-thumbnails box art (Xbox 360, PS3, PS2), matched by name (fuzzy)
//! - PS2 covers by disc serial (the set PCSX2's own cover downloader uses)
//! - Switch icons/banners by title ID (api.nlib.cc)
//! - whatever came with the game (GOD thumbnail, PS3 ICON0)
//!
//! Files
//! - theme/covers/<console>/<key>.png|jpg   covers you picked (always win)
//! - theme/cache/covers/<console>/<key>.*    automatic downloads
//! - theme/cache/libretro/<repo>.txt         cached name lists (refreshed every 14 days)
//! - theme/cache/cover-tried.txt             games already auto-searched

use crate::systems::System;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct GameRef {
    pub system: System,
    pub name: String,
    pub stem: String,
    pub id: Option<String>,
    pub path: PathBuf,
    pub embedded: Option<Vec<u8>>,
}

impl GameRef {
    pub fn key(&self) -> String {
        sanitize(self.id.as_deref().unwrap_or(&self.name))
    }
}

pub enum Job {
    Auto(GameRef),
    Candidates(u64, GameRef),
}

pub enum Msg {
    /// A cover was saved for the game at this path.
    Saved(PathBuf),
    Candidates(u64, Vec<(String, Vec<u8>)>),
    CandidatesDone(u64),
    Progress { done: usize, queued: usize },
}

pub fn manual_dir(theme: &Path, s: System) -> PathBuf {
    theme.join("covers").join(s.key())
}
pub fn auto_dir(theme: &Path, s: System) -> PathBuf {
    theme.join("cache").join("covers").join(s.key())
}
fn tried_file(theme: &Path) -> PathBuf {
    theme.join("cache").join("cover-tried.txt")
}

pub fn sanitize(s: &str) -> String {
    s.chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect::<String>().trim().to_string()
}

pub fn load_tried(theme: &Path) -> HashSet<String> {
    std::fs::read_to_string(tried_file(theme)).map(|s| s.lines().map(String::from).collect()).unwrap_or_default()
}

pub fn tried_key(g: &GameRef) -> String {
    format!("{}|{}", g.system.key(), g.key())
}

/// Write `bytes` as <dir>/<key>.<png|jpg>, replacing any older file for that key.
pub fn save_image(dir: &Path, key: &str, bytes: &[u8]) -> Option<PathBuf> {
    let ext = match image::guess_format(bytes).ok()? {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpg",
        _ => return None,
    };
    std::fs::create_dir_all(dir).ok()?;
    remove_image(dir, key);
    let p = dir.join(format!("{key}.{ext}"));
    std::fs::write(&p, bytes).ok()?;
    Some(p)
}

pub fn remove_image(dir: &Path, key: &str) {
    for e in ["png", "jpg", "jpeg"] {
        let _ = std::fs::remove_file(dir.join(format!("{key}.{e}")));
    }
}

// ------------------------------------------------------------------ worker

pub fn spawn(theme: PathBuf, auto_rx: Receiver<Job>, pri_rx: Receiver<Job>, tx: Sender<Msg>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let net = Net::new(theme.clone());
        let mut tried = load_tried(&theme);
        let mut done = 0usize;
        let mut pending: Vec<GameRef> = Vec::new();
        loop {
            // Picker requests jump the queue.
            while let Ok(job) = pri_rx.try_recv() {
                handle(&net, job, &tx, &ctx, &theme, &mut tried, &mut done, &mut pending);
            }
            match auto_rx.recv_timeout(Duration::from_millis(80)) {
                Ok(job) => handle(&net, job, &tx, &ctx, &theme, &mut tried, &mut done, &mut pending),
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(g) = pending.pop() {
                        auto_one(&net, &g, &tx, &theme, &mut tried);
                        done += 1;
                        let _ = tx.send(Msg::Progress { done, queued: pending.len() });
                        ctx.request_repaint();
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn handle(
    net: &Net,
    job: Job,
    tx: &Sender<Msg>,
    ctx: &eframe::egui::Context,
    _theme: &Path,
    tried: &mut HashSet<String>,
    _done: &mut usize,
    pending: &mut Vec<GameRef>,
) {
    match job {
        Job::Auto(g) => {
            if !tried.contains(&tried_key(&g)) && !pending.iter().any(|p| p.path == g.path) {
                pending.insert(0, g);
                let _ = tx.send(Msg::Progress { done: *_done, queued: pending.len() });
            }
        }
        Job::Candidates(req, g) => {
            let mut out: Vec<(String, Vec<u8>)> = Vec::new();
            if let Some(e) = &g.embedded {
                out.push(("From the game files".into(), e.clone()));
            }
            for c in net.candidates(&g, 8) {
                if let Some(b) = net.get_image(&c.url) {
                    out.push((c.label, b));
                    // Stream partial results so the picker fills in as they arrive.
                    let _ = tx.send(Msg::Candidates(req, out.clone()));
                    ctx.request_repaint();
                }
            }
            let _ = tx.send(Msg::Candidates(req, out));
            let _ = tx.send(Msg::CandidatesDone(req));
        }
    }
    ctx.request_repaint();
}

fn auto_one(net: &Net, g: &GameRef, tx: &Sender<Msg>, theme: &Path, tried: &mut HashSet<String>) {
    let cands = net.candidates(g, 4);
    for c in cands.iter().filter(|c| c.auto_ok) {
        if let Some(b) = net.get_image(&c.url) {
            if save_image(&auto_dir(theme, g.system), &g.key(), &b).is_some() {
                let _ = tx.send(Msg::Saved(g.path.clone()));
            }
            break;
        }
    }
    let k = tried_key(g);
    if tried.insert(k.clone()) {
        let _ = std::fs::create_dir_all(theme.join("cache"));
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(tried_file(theme)) {
            let _ = writeln!(f, "{k}");
        }
    }
}

// ------------------------------------------------------------------ sources

pub struct Cand {
    pub label: String,
    pub url: String,
    /// Confident enough to use without asking.
    pub auto_ok: bool,
}

struct Net {
    agent: ureq::Agent,
    theme: PathBuf,
    lists: std::sync::Mutex<std::collections::HashMap<&'static str, Vec<String>>>,
}

fn repo(s: System) -> &'static str {
    s.cover_repo().unwrap_or("")
}

impl Net {
    fn new(theme: PathBuf) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(25)))
            .user_agent(concat!("nxe-launcher/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self { agent, theme, lists: Default::default() }
    }

    fn get(&self, url: &str) -> Option<Vec<u8>> {
        let mut r = self.agent.get(url).call().ok()?;
        r.body_mut().with_config().limit(20 * 1024 * 1024).read_to_vec().ok()
    }

    fn get_image(&self, url: &str) -> Option<Vec<u8>> {
        let b = self.get(url)?;
        image::load_from_memory(&b).ok()?;
        Some(b)
    }

    /// Box-art file names in a libretro-thumbnails repo (cached on disk).
    fn libretro_names(&self, s: System) -> Vec<String> {
        let r = repo(s);
        if let Some(v) = self.lists.lock().unwrap().get(r) {
            return v.clone();
        }
        let cache = self.theme.join("cache").join("libretro").join(format!("{r}.txt"));
        let fresh = std::fs::metadata(&cache)
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().unwrap_or_default() < Duration::from_secs(14 * 86400))
            .unwrap_or(false);
        let read_cache = || std::fs::read_to_string(&cache).map(|s| s.lines().map(String::from).collect::<Vec<_>>()).ok();
        let names = if fresh { read_cache() } else { None }.or_else(|| {
            let fetched = self.fetch_libretro_list(r);
            match fetched {
                Some(v) if !v.is_empty() => {
                    let _ = std::fs::create_dir_all(cache.parent().unwrap());
                    let _ = std::fs::write(&cache, v.join("\n"));
                    Some(v)
                }
                _ => read_cache(),
            }
        });
        let names = names.unwrap_or_default();
        self.lists.lock().unwrap().insert(r, names.clone());
        names
    }

    fn fetch_libretro_list(&self, r: &str) -> Option<Vec<String>> {
        let root: serde_json::Value =
            serde_json::from_slice(&self.get(&format!("https://api.github.com/repos/libretro-thumbnails/{r}/git/trees/master"))?).ok()?;
        let sha = root["tree"]
            .as_array()?
            .iter()
            .find(|e| e["path"] == "Named_Boxarts")
            .and_then(|e| e["sha"].as_str())?
            .to_string();
        let t: serde_json::Value =
            serde_json::from_slice(&self.get(&format!("https://api.github.com/repos/libretro-thumbnails/{r}/git/trees/{sha}"))?).ok()?;
        Some(
            t["tree"]
                .as_array()?
                .iter()
                .filter_map(|e| e["path"].as_str())
                .filter_map(|p| p.strip_suffix(".png"))
                .map(String::from)
                .collect(),
        )
    }

    fn candidates(&self, g: &GameRef, max_fuzzy: usize) -> Vec<Cand> {
        let mut out = Vec::new();
        match g.system {
            System::Switch => {
                if let Some(id) = g.id.as_ref().filter(|i| i.len() == 16 && i.chars().all(|c| c.is_ascii_hexdigit())) {
                    // Updates/DLC share the base game's art: base ID ends in 000.
                    let base = format!("{}000", &id[..13]);
                    out.push(Cand { label: "Game icon".into(), url: format!("https://api.nlib.cc/nx/{base}/icon"), auto_ok: true });
                    out.push(Cand { label: "Banner".into(), url: format!("https://api.nlib.cc/nx/{base}/banner"), auto_ok: false });
                }
            }
            System::Ps2 => {
                if let Some(serial) = g.id.as_ref().filter(|i| is_ps2_serial(i)) {
                    out.push(Cand {
                        label: format!("Cover · {serial}"),
                        url: format!("https://raw.githubusercontent.com/xlenore/ps2-covers/main/covers/default/{serial}.jpg"),
                        auto_ok: true,
                    });
                }
            }
            _ => {}
        }
        if g.system == System::Gc {
            if let Some(id) = g.id.as_ref().filter(|i| i.len() == 6) {
                // GameTDB box art by disc ID, region from the 4th character.
                let regions: &[&str] = match id.as_bytes()[3] {
                    b'E' | b'N' => &["US"],
                    b'J' => &["JA"],
                    b'P' | b'D' | b'F' | b'S' | b'I' | b'X' | b'Y' => &["EN", "FR", "DE", "ES", "IT"],
                    _ => &["US", "EN", "JA"],
                };
                for r in regions.iter().take(2) {
                    out.push(Cand { label: format!("Cover · {id}"), url: format!("https://art.gametdb.com/wii/cover/{r}/{id}.png"), auto_ok: true });
                }
            }
        }
        if g.system.cover_repo().is_some() {
            let names = self.libretro_names(g.system);
            for (name, score) in best_matches(&g.name, &g.stem, &names, max_fuzzy) {
                out.push(Cand {
                    label: name.clone(),
                    url: format!(
                        "https://raw.githubusercontent.com/libretro-thumbnails/{}/master/Named_Boxarts/{}.png",
                        repo(g.system),
                        pct(&name)
                    ),
                    auto_ok: score >= 0.6,
                });
            }
        }
        out
    }
}

fn is_ps2_serial(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10 && b[4] == b'-' && b[..4].iter().all(|c| c.is_ascii_uppercase()) && b[5..].iter().all(|c| c.is_ascii_digit())
}

fn pct(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

// ------------------------------------------------------------------ fuzzy matching

fn norm_tokens(s: &str) -> Vec<String> {
    let base = crate::library::clean_name(s).to_lowercase().replace('&', " and ");
    let base: String = base.chars().map(|c| if c.is_alphanumeric() { c } else { ' ' }).collect();
    let roman = |t: &str| match t {
        "ii" => "2".to_string(),
        "iii" => "3".to_string(),
        "iv" => "4".to_string(),
        "v" => "5".to_string(),
        _ => t.to_string(),
    };
    base.split_whitespace().filter(|t| !matches!(*t, "the" | "a" | "of")).map(roman).collect()
}

fn dice(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let sa: HashSet<&String> = a.iter().collect();
    let sb: HashSet<&String> = b.iter().collect();
    let inter = sa.intersection(&sb).count() as f32;
    let mut s = 2.0 * inter / (sa.len() + sb.len()) as f32;
    if a == b {
        s += 0.2;
    }
    // Same leading word matters a lot for titles.
    if a[0] == b[0] {
        s += 0.05;
    }
    s
}

pub fn best_matches_pub(name: &str, stem: &str, list: &[String], n: usize) -> Vec<(String, f32)> {
    best_matches(name, stem, list, n)
}

/// Top matches, one per distinct title, preferring USA then Europe releases.
fn best_matches(name: &str, stem: &str, list: &[String], n: usize) -> Vec<(String, f32)> {
    let qa = norm_tokens(name);
    let qb = norm_tokens(stem);
    let mut scored: Vec<(String, f32)> = list
        .iter()
        .map(|c| {
            let t = norm_tokens(c);
            let mut s = dice(&qa, &t).max(dice(&qb, &t));
            if c.eq_ignore_ascii_case(stem) {
                s += 1.0;
            }
            if c.contains("(USA") {
                s += 0.02;
            } else if c.contains("Europe") {
                s += 0.01;
            }
            (c.clone(), s)
        })
        .filter(|(_, s)| *s > 0.3)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let mut seen = HashSet::new();
    scored.retain(|(c, _)| seen.insert(norm_tokens(c)));
    scored.truncate(n);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches() {
        let list: Vec<String> = [
            "Shadow of the Colossus (USA)",
            "Shadow of the Colossus (Europe)",
            "Shadow Hearts (USA)",
            "Gran Turismo 4 (USA)",
            "God of War II (USA)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let m = best_matches("Shadow of the Colossus", "Shadow of the Colossus (USA)", &list, 3);
        assert_eq!(m[0].0, "Shadow of the Colossus (USA)");
        let m = best_matches("God of War 2", "God of War 2", &list, 3);
        assert_eq!(m[0].0, "God of War II (USA)");
        assert!(m[0].1 >= 0.6);
    }
}
