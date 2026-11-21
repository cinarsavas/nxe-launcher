//! Cover art: automatic download in the background and candidate lists for the
//! "choose a cover" picker.
//!
//! Sources
//! - libretro-thumbnails box art, title screens and screenshots, matched by name (fuzzy)
//! - PS2 covers by disc serial (the set PCSX2's own cover downloader uses)
//! - GameCube covers by disc ID (GameTDB)
//! - Switch: eShop icon, banner and screenshots from blawar/titledb, by title ID or name,
//!   plus api.nlib.cc by title ID
//! - PC: a port's own art from its repository, the .exe's icon, the original game's box art,
//!   and Steam store art by name
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
                out.push(("From the game files\tBuilt-in art".into(), e.clone()));
            }
            let mut seen = HashSet::new();
            for c in net.candidates(&g, 12) {
                // Different sources often serve the very same picture.
                if let Some(b) = net.get_image(&c.url).filter(|b| seen.insert(hash(b))) {
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
    switch: std::sync::Mutex<Option<std::sync::Arc<Vec<SwTitle>>>>,
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
        Self { agent, theme, lists: Default::default(), switch: Default::default() }
    }

    fn get(&self, url: &str) -> Option<Vec<u8>> {
        let mut r = self.agent.get(url).call().ok()?;
        r.body_mut().with_config().limit(20 * 1024 * 1024).read_to_vec().ok()
    }

    fn get_image(&self, url: &str) -> Option<Vec<u8>> {
        if let Some(p) = url.strip_prefix("exe-icon:") {
            return exe_icon(Path::new(p)).filter(|b| image::load_from_memory(b).map(|i| i.width() >= 64).unwrap_or(false));
        }
        let b = self.get(url)?;
        image::load_from_memory(&b).ok()?;
        Some(b)
    }

    /// Box-art file names in a libretro-thumbnails repo (cached on disk).
    fn libretro_names(&self, s: System) -> Vec<String> {
        self.libretro_names_in(repo(s))
    }

    fn libretro_names_in(&self, r: &'static str) -> Vec<String> {
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
        let lr = |r: &str, name: &str| format!("https://raw.githubusercontent.com/libretro-thumbnails/{r}/master/Named_Boxarts/{}.png", pct(name));
        match g.system {
            System::Switch => {
                let valid = g.id.as_ref().filter(|i| i.len() == 16 && i.chars().all(|c| c.is_ascii_hexdigit()));
                let base = valid.map(|id| format!("{}000", id[..13].to_uppercase()));
                let idx = self.switch_index();
                // By title ID first (exact), then by name.
                let mut hits: Vec<(&SwTitle, bool)> = vec![];
                if let Some(b) = &base {
                    if let Some(t) = idx.iter().find(|t| &t.id == b) {
                        hits.push((t, true));
                    }
                }
                let names: Vec<String> = idx.iter().map(|t| t.name.clone()).collect();
                // With an exact ID hit, look for related titles by the real name, not the file name.
                let (qn, qs) = match hits.first() {
                    Some((t, _)) => (t.name.clone(), t.name.clone()),
                    None => (g.name.clone(), g.stem.clone()),
                };
                for (n, score) in best_matches(&qn, &qs, &names, max_fuzzy.min(4)) {
                    if let Some(t) = idx.iter().find(|t| t.name == n && !t.id.is_empty()) {
                        if !hits.iter().any(|(h, _)| h.id == t.id) {
                            hits.push((t, score >= 0.75 && base.is_none()));
                        }
                    }
                }
                if let Some(b) = &base {
                    out.push(Cand { label: format!("nlib\tGame icon · {b}"), url: format!("https://api.nlib.cc/nx/{b}/icon"), auto_ok: true });
                }
                for (i, (t, sure)) in hits.iter().enumerate() {
                    if !t.icon.is_empty() {
                        out.push(Cand { label: format!("eShop icon\t{}", t.name), url: t.icon.clone(), auto_ok: *sure });
                    }
                    if !t.banner.is_empty() {
                        out.push(Cand { label: format!("eShop banner\t{}", t.name), url: t.banner.clone(), auto_ok: false });
                    }
                    if i == 0 {
                        for (k, sc) in t.shots.iter().take(3).enumerate() {
                            out.push(Cand { label: format!("eShop screenshot {}\t{}", k + 1, t.name), url: sc.clone(), auto_ok: false });
                        }
                    }
                }
                if let Some(b) = &base {
                    out.push(Cand { label: format!("nlib\tBanner · {b}"), url: format!("https://api.nlib.cc/nx/{b}/banner"), auto_ok: false });
                }
            }
            System::Ps2 => {
                if let Some(serial) = g.id.as_ref().filter(|i| is_ps2_serial(i)) {
                    out.push(Cand {
                        label: format!("PS2 covers\t{serial}"),
                        url: format!("https://raw.githubusercontent.com/xlenore/ps2-covers/main/covers/default/{serial}.jpg"),
                        auto_ok: true,
                    });
                }
            }
            System::Gc => {
                if let Some(id) = g.id.as_ref().filter(|i| i.len() == 6) {
                    // GameTDB box art by disc ID, region from the 4th character.
                    let regions: &[&str] = match id.as_bytes()[3] {
                        b'E' | b'N' => &["US"],
                        b'J' => &["JA"],
                        b'P' | b'D' | b'F' | b'S' | b'I' | b'X' | b'Y' => &["EN", "FR", "DE", "ES", "IT"],
                        _ => &["US", "EN", "JA"],
                    };
                    for r in regions.iter().take(2) {
                        out.push(Cand { label: format!("GameTDB\t{id} ({r})"), url: format!("https://art.gametdb.com/wii/cover/{r}/{id}.png"), auto_ok: true });
                    }
                }
            }
            System::Pc => {
                // A recompilation's own art (its app icon / logo) comes first.
                if let Some((repo, paths)) = port_art(g) {
                    for p in paths {
                        out.push(Cand { label: format!("Project art\t{repo}"), url: format!("https://raw.githubusercontent.com/{repo}/HEAD/{p}"), auto_ok: true });
                    }
                }
                if g.path.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false) {
                    out.push(Cand { label: "From the program\tApp icon".into(), url: format!("exe-icon:{}", g.path.display()), auto_ok: false });
                }
                // A PC port also gets the box art of the game it's a port of.
                let key = squash(&format!("{} {}", g.name, g.stem));
                let port = PC_PORTS.iter().find(|(keys, _, _)| keys.split('|').any(|k| key.contains(k)));
                if let Some((_, r, title)) = port {
                    let names = self.libretro_names_in(r);
                    for (name, score) in best_matches(title, title, &names, max_fuzzy.max(3)) {
                        out.push(Cand { label: format!("Original box art\t{name}"), url: lr(r, &name), auto_ok: score >= 0.6 });
                    }
                }
                // Steam store art by name (portrait library cover, header, hero).
                for (i, (appid, name, score)) in self.steam_search(&g.name).into_iter().enumerate() {
                    let sure = port.is_none() && i == 0 && score >= 0.75;
                    let base = format!("https://cdn.akamai.steamstatic.com/steam/apps/{appid}");
                    out.push(Cand { label: format!("Steam\t{name}"), url: format!("{base}/library_600x900_2x.jpg"), auto_ok: sure });
                    if i < 2 {
                        out.push(Cand { label: format!("Steam header\t{name}"), url: format!("{base}/header.jpg"), auto_ok: false });
                    }
                    if i == 0 {
                        out.push(Cand { label: format!("Steam artwork\t{name}"), url: format!("{base}/library_hero.jpg"), auto_ok: false });
                    }
                }
            }
            _ => {}
        }
        if let Some(r) = g.system.cover_repo() {
            let names = self.libretro_names(g.system);
            for (k, (name, score)) in best_matches(&g.name, &g.stem, &names, max_fuzzy).into_iter().enumerate() {
                out.push(Cand { label: format!("libretro\t{name}"), url: lr(r, &name), auto_ok: score >= 0.6 });
                if k == 0 && max_fuzzy > 4 {
                    for (kind, what) in [("Named_Titles", "Title screen"), ("Named_Snaps", "Screenshot")] {
                        out.push(Cand {
                            label: format!("libretro {what}\t{name}"),
                            url: format!("https://raw.githubusercontent.com/libretro-thumbnails/{r}/master/{kind}/{}.png", pct(&name)),
                            auto_ok: false,
                        });
                    }
                }
            }
        }
        // Auto-pick in the order above, but never the same URL twice.
        let mut seen = HashSet::new();
        out.retain(|c| seen.insert(c.url.clone()));
        out
    }

    /// Switch titles from blawar/titledb (the list Switch homebrew tools use). The 90 MB
    /// list is downloaded once and boiled down to a small index; refreshed monthly.
    fn switch_index(&self) -> std::sync::Arc<Vec<SwTitle>> {
        let mut slot = self.switch.lock().unwrap();
        if let Some(v) = slot.as_ref() {
            return v.clone();
        }
        let dir = self.theme.join("cache");
        let idx = dir.join("switch-titles.tsv");
        let fresh = std::fs::metadata(&idx)
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().unwrap_or_default() < Duration::from_secs(30 * 86400))
            .unwrap_or(false);
        if !fresh {
            if let Some(rows) = self.fetch_titledb(&dir) {
                let _ = std::fs::write(&idx, rows);
            }
        }
        let v: Vec<SwTitle> = std::fs::read_to_string(&idx)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let mut f = l.split('\t');
                Some(SwTitle {
                    id: f.next()?.into(),
                    name: f.next()?.into(),
                    icon: f.next()?.into(),
                    banner: f.next()?.into(),
                    shots: f.next().unwrap_or("").split('|').filter(|s| !s.is_empty()).map(String::from).collect(),
                })
            })
            .collect();
        let v = std::sync::Arc::new(v);
        *slot = Some(v.clone());
        v
    }

    fn fetch_titledb(&self, dir: &Path) -> Option<String> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct E {
            id: Option<String>,
            name: Option<String>,
            icon_url: Option<String>,
            banner_url: Option<String>,
            screenshots: Option<Vec<String>>,
        }
        std::fs::create_dir_all(dir).ok()?;
        let tmp = dir.join("titledb.json.part");
        {
            let mut r = self.agent.get("https://raw.githubusercontent.com/blawar/titledb/master/US.en.json").call().ok()?;
            if r.status() != 200 {
                return None;
            }
            let mut f = std::fs::File::create(&tmp).ok()?;
            std::io::copy(&mut r.body_mut().as_reader(), &mut f).ok()?;
        }
        let f = std::io::BufReader::new(std::fs::File::open(&tmp).ok()?);
        let all: Option<std::collections::HashMap<String, E>> = serde_json::from_reader(f).ok();
        let _ = std::fs::remove_file(&tmp);
        let all = all?;
        let clean = |s: &str| -> String { s.replace(['\t', '\n', '\r', '|'], " ").replace(['™', '®'], "") };
        let mut rows: Vec<String> = all
            .into_values()
            .filter_map(|e| {
                let id = e.id?.to_uppercase();
                let name = clean(&e.name?);
                if id.len() != 16 || !id.ends_with("000") {
                    return None;
                }
                let shots: Vec<String> = e.screenshots.unwrap_or_default().into_iter().take(3).map(|s: String| clean(&s)).collect();
                Some(format!("{id}\t{}\t{}\t{}\t{}", name.trim(), clean(&e.icon_url.unwrap_or_default()), clean(&e.banner_url.unwrap_or_default()), shots.join("|")))
            })
            .collect();
        rows.sort();
        rows.dedup();
        Some(rows.join("\n"))
    }

    /// Steam store search by name: (app id, name, match score), best first.
    fn steam_search(&self, name: &str) -> Vec<(u64, String, f32)> {
        let q = crate::library::clean_name(name);
        let url = format!("https://store.steampowered.com/api/storesearch/?term={}&cc=us&l=en", pct(&q));
        let Some(b) = self.get(&url) else { return vec![] };
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) else { return vec![] };
        let qt = norm_tokens(&q);
        let mut out: Vec<(u64, String, f32)> = v["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|i| Some((i["id"].as_u64()?, i["name"].as_str()?.to_string())))
                    .map(|(id, n)| {
                        let s = dice(&qt, &norm_tokens(&n));
                        (id, n, s)
                    })
                    .filter(|(_, _, s)| *s > 0.3)
                    .collect()
            })
            .unwrap_or_default();
        out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        out.truncate(3);
        out
    }
}

pub struct SwTitle {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub banner: String,
    pub shots: Vec<String>,
}

/// Art that ships inside a port's own repository (checked by hand).
const PORT_ART: &[(&str, &[&str])] = &[
    ("Zelda64Recomp/Zelda64Recomp", &["icons/512.png"]),
    ("HarbourMasters/Shipwright", &["soh/macosx/sohIcon.png", "soh/icon.jpg"]),
    ("HarbourMasters/2ship2harkinian", &["mm/linux/2s2hIcon.png"]),
    ("HarbourMasters/SpaghettiKart", &["icon.png"]),
    ("HarbourMasters/Starship", &["logo.png"]),
    ("BanjoRecomp/BanjoRecomp", &["icons/app.png"]),
];

/// The Get Ports entry a PC game belongs to (installed by us, or by folder/exe name).
fn port_art(g: &GameRef) -> Option<(&'static str, &'static [&'static str])> {
    let key = squash(&format!("{} {}", g.path.display(), g.name));
    let p = crate::net::PORTS.iter().find(|p| g.path.starts_with(crate::net::ports_dir().join(p.folder)) || key.contains(&squash(p.folder)))?;
    PORT_ART.iter().find(|(r, _)| *r == p.repo).copied()
}

/// The biggest icon embedded in a Windows .exe, as PNG bytes.
pub fn exe_icon(path: &Path) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut at = |off: u64, len: usize| -> Option<Vec<u8>> {
        if len > 16 << 20 {
            return None;
        }
        f.seek(SeekFrom::Start(off)).ok()?;
        let mut b = vec![0; len];
        f.read_exact(&mut b).ok()?;
        Some(b)
    };
    let u16_ = |b: &[u8], o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_ = |b: &[u8], o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let dos = at(0, 64)?;
    if &dos[..2] != b"MZ" {
        return None;
    }
    let pe = u32_(&dos, 0x3c) as u64;
    let hdr = at(pe, 24)?;
    if &hdr[..4] != b"PE\0\0" {
        return None;
    }
    let nsec = u16_(&hdr, 6) as usize;
    let optsz = u16_(&hdr, 20) as usize;
    let opt = at(pe + 24, optsz)?;
    let dd = match u16_(&opt, 0) {
        0x10b => 96,
        0x20b => 112,
        _ => return None,
    };
    if opt.len() < dd + 24 {
        return None;
    }
    let rsrc_rva = u32_(&opt, dd + 16);
    if rsrc_rva == 0 {
        return None;
    }
    let secs = at(pe + 24 + optsz as u64, nsec * 40)?;
    let to_off = |rva: u32| -> Option<u64> {
        (0..nsec).find_map(|i| {
            let s = &secs[i * 40..];
            let (va, vs, raw) = (u32_(s, 12), u32_(s, 8).max(u32_(s, 16)), u32_(s, 20));
            (rva >= va && rva < va + vs).then(|| (raw + rva - va) as u64)
        })
    };
    let base = to_off(rsrc_rva)?;
    // Whole resource section is small; read up to 32 MB of it.
    let sec = (0..nsec).map(|i| &secs[i * 40..]).find(|s| rsrc_rva >= u32_(s, 12) && rsrc_rva < u32_(s, 12) + u32_(s, 8).max(u32_(s, 16)))?;
    let len = (u32_(sec, 16).max(u32_(sec, 8)) - (rsrc_rva - u32_(sec, 12))).min(32 << 20) as usize;

    let mut f2 = std::fs::File::open(path).ok()?;
    f2.seek(SeekFrom::Start(base)).ok()?;
    let mut rs = vec![0; len];
    let n = f2.read(&mut rs).ok()?;
    rs.truncate(n);
    // Directory entries: (id or name, offset, is_dir).
    let entries = |o: usize| -> Vec<(u32, usize, bool)> {
        if o + 16 > rs.len() {
            return vec![];
        }
        let n = u16_(&rs, o + 12) as usize + u16_(&rs, o + 14) as usize;
        (0..n)
            .filter_map(|i| {
                let e = o + 16 + i * 8;
                (e + 8 <= rs.len()).then(|| {
                    let d = u32_(&rs, e + 4);
                    (u32_(&rs, e), (d & 0x7fff_ffff) as usize, d & 0x8000_0000 != 0)
                })
            })
            .collect()
    };
    // Follow type -> name -> first language down to the data.
    let leaf = |ty: u32, id: Option<u32>| -> Option<Vec<u8>> {
        let (_, o, _) = entries(0).into_iter().find(|e| e.0 == ty && e.2)?;
        let (_, o, _) = entries(o).into_iter().find(|e| e.2 && id.map(|i| e.0 == i).unwrap_or(true))?;
        let (_, o, dir) = entries(o).into_iter().next()?;
        if dir || o + 8 > rs.len() {
            return None;
        }
        let (rva, size) = (u32_(&rs, o), u32_(&rs, o + 4) as usize);
        let start = (rva - rsrc_rva) as usize;
        rs.get(start..start + size).map(|s| s.to_vec())
    };
    let group = leaf(14, None)?;
    let count = u16_(&group, 4) as usize;
    let best = (0..count)
        .filter_map(|i| group.get(6 + i * 14..6 + i * 14 + 14))
        .max_by_key(|e| {
            let w = if e[0] == 0 { 256 } else { e[0] as u32 };
            (w, u16_(e, 6))
        })?
        .to_vec();
    let data = leaf(3, Some(u16_(&best, 12) as u32))?;
    if data.starts_with(b"\x89PNG") {
        return Some(data);
    }
    // Plain bitmap icon: wrap it in an .ico so the image crate can read it.
    let mut ico = vec![0, 0, 1, 0, 1, 0];
    ico.extend_from_slice(&best[..8]);
    ico.extend_from_slice(&(data.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&data);
    let img = image::load_from_memory_with_format(&ico, image::ImageFormat::Ico).ok()?;
    let mut png = std::io::Cursor::new(vec![]);
    img.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

fn hash(b: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

const N64: &str = "Nintendo_-_Nintendo_64";
const GC: &str = "Nintendo_-_GameCube";
const MD: &str = "Sega_-_Mega_Drive_-_Genesis";
const SNES: &str = "Nintendo_-_Super_Nintendo_Entertainment_System";
const PS1: &str = "Sony_-_PlayStation";
const PS2: &str = "Sony_-_PlayStation_2";
const DC: &str = "Sega_-_Dreamcast";
const GBA: &str = "Nintendo_-_Game_Boy_Advance";
const NES: &str = "Nintendo_-_Nintendo_Entertainment_System";

/// Known PC ports: words in the exe/folder name (squashed, any of `|`), the original console's
/// thumbnail repo, and the original game's title to look up there.
const PC_PORTS: &[(&str, &str, &str)] = &[
    ("zelda64|majora|2ship", N64, "Legend of Zelda, The - Majora's Mask (USA)"),
    ("ocarina|soh|shipofharkinian|harkinian", N64, "Legend of Zelda, The - Ocarina of Time (USA)"),
    ("starfox64|starship|sf64", N64, "Star Fox 64 (USA)"),
    ("banjo|bkrecomp", N64, "Banjo-Kazooie (USA)"),
    ("mariokart64|mk64|spaghettikart|spaghetti", N64, "Mario Kart 64 (USA)"),
    ("sm64|supermario64|render96|mario64|ghostship", N64, "Super Mario 64 (USA)"),
    ("dk64|donkeykong64", N64, "Donkey Kong 64 (USA)"),
    ("goemon|mnsg", N64, "Mystical Ninja Starring Goemon (USA)"),
    ("perfectdark", N64, "Perfect Dark (USA)"),
    ("drmario64", N64, "Dr. Mario 64 (USA)"),
    ("paperma|papermario", N64, "Paper Mario (USA)"),
    ("dusk|twilightprincess", GC, "Legend of Zelda, The - Twilight Princess (USA)"),
    ("windwaker", GC, "Legend of Zelda, The - The Wind Waker (USA)"),
    ("sonic3air|sonic3", MD, "Sonic The Hedgehog 3 (USA)"),
    ("sonic2", MD, "Sonic The Hedgehog 2 (World)"),
    ("smw|supermarioworld", SNES, "Super Mario World (USA)"),
    ("zelda3|linktothepast|alttp", SNES, "Legend of Zelda, The - A Link to the Past (USA)"),
    ("supermetroid", SNES, "Super Metroid (Japan, USA) (En,Ja)"),
    ("re3|gta3", PS2, "Grand Theft Auto III (USA)"),
    ("revc|vicecity|vcs", PS2, "Grand Theft Auto - Vice City (Europe) (En,Fr,De,Es,It) (v2.03)"),
    ("sotn|symphony", PS1, "Castlevania - Symphony of the Night (USA)"),
    ("crash", PS1, "Crash Bandicoot (USA)"),
    ("spyro", PS1, "Spyro the Dragon (USA)"),
    ("sonicadventure|sadx", DC, "Sonic Adventure (USA)"),
    ("minishcap|tmc", GBA, "Legend of Zelda, The - The Minish Cap (USA)"),
    ("smb1|supermariobros", NES, "Super Mario Bros. (World)"),
];

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

// ------------------------------------------------------------------ pictures for a game

/// Title screen and gameplay screenshot of a game from libretro-thumbnails (the same project
/// the covers come from). PC ports use the pictures of the game they are a port of.
pub fn picture_urls(theme: PathBuf, system: System, name: &str, stem: &str) -> Vec<String> {
    let (repo, title): (&'static str, String) = if system == System::Pc {
        let key = squash(&format!("{name} {stem}"));
        match PC_PORTS.iter().find(|(keys, _, _)| keys.split('|').any(|k| key.contains(k))) {
            Some((_, r, t)) => (*r, t.to_string()),
            None => return vec![],
        }
    } else {
        match system.cover_repo() {
            Some(r) => (r, name.to_string()),
            None => return vec![],
        }
    };
    let net = Net::new(theme);
    let names = net.libretro_names_in(repo);
    let Some((best, score)) = best_matches(&title, if system == System::Pc { &title } else { stem }, &names, 1).into_iter().next() else { return vec![] };
    if score < 0.6 {
        return vec![];
    }
    ["Named_Titles", "Named_Snaps"]
        .iter()
        .map(|kind| format!("https://raw.githubusercontent.com/libretro-thumbnails/{repo}/master/{kind}/{}.png", pct(&best)))
        .collect()
}

/// Screenshots from a project's README (badges, logos and the like left out).
pub fn readme_images(repo: &str, readme: &str) -> Vec<String> {
    let mut urls = vec![];
    let mut push = |u: &str| {
        let u = u.trim().trim_matches(|c| c == '"' || c == '\'').split_whitespace().next().unwrap_or("").to_string();
        if u.is_empty() {
            return;
        }
        let full = if u.starts_with("http") {
            u.replace("github.com/", "raw.githubusercontent.com/").replacen("/blob/", "/", 1).replace("raw.githubusercontent.com/user-attachments", "github.com/user-attachments")
        } else {
            format!("https://raw.githubusercontent.com/{repo}/HEAD/{}", u.trim_start_matches("./").trim_start_matches('/'))
        };
        let l = full.to_lowercase();
        let bad = ["discord", "badge", "shields.io", "logo", "poweredby", "powered_by", "widget", "sponsor", "ko-fi", "patreon", "icon", ".svg", ".gif", "button"];
        let image = l.contains("user-attachments") || [".png", ".jpg", ".jpeg", ".webp"].iter().any(|e| l.split('?').next().unwrap_or("").ends_with(e));
        if image && !bad.iter().any(|b| l.contains(b)) && !urls.contains(&full) {
            urls.push(full);
        }
    };
    // ![alt](url "title")
    let mut rest = readme;
    while let Some(i) = rest.find("](") {
        let after = &rest[i + 2..];
        if rest[..i].rfind("![").map(|j| !rest[j..i].contains('\n')).unwrap_or(false) {
            if let Some(end) = after.find(')') {
                push(&after[..end]);
            }
        }
        rest = after;
    }
    // <img src="url">
    let mut rest = readme;
    while let Some(i) = rest.find("src=") {
        let after = &rest[i + 4..];
        let q = after.chars().next().unwrap_or('"');
        if q == '"' || q == '\'' {
            if let Some(end) = after[1..].find(q) {
                push(&after[1..1 + end]);
            }
        }
        rest = after;
    }
    urls.truncate(4);
    urls
}

#[cfg(test)]
mod pic_tests {
    #[test]
    fn readme_filter() {
        let md = "![Discord Invitation](https://discordapp.com/api/guilds/1/widget.png?style=banner2 'N64')\n![Gyro](docs/deck_gyro_1.jpg)\n<img alt=\"x\" src=\"./docs/poweredbylus.lightmode.png\">\n<img src=\"https://github.com/user-attachments/assets/abc\" width=600>";
        let v = super::readme_images("Zelda64Recomp/Zelda64Recomp", md);
        assert_eq!(v, vec!["https://raw.githubusercontent.com/Zelda64Recomp/Zelda64Recomp/HEAD/docs/deck_gyro_1.jpg".to_string(), "https://github.com/user-attachments/assets/abc".to_string()]);
    }
}

#[cfg(test)]
mod net_tests {
    use super::*;
    fn g(system: System, name: &str, id: Option<&str>, path: &str) -> GameRef {
        GameRef { system, name: name.into(), stem: name.into(), id: id.map(String::from), path: path.into(), embedded: None }
    }
    #[test]
    #[ignore]
    fn sources_online() {
        let net = Net::new(std::env::temp_dir().join("nxe_theme_test"));
        for gr in [
            g(System::Switch, "Super Mario Odyssey", None, "x.nsp"),
            g(System::Switch, "whatever", Some("0100000000010000"), "x.nsp"),
            g(System::Pc, "Hollow Knight", None, "C:/Games/Hollow Knight/hollow_knight.exe"),
            g(System::Pc, "Zelda64Recompiled", None, "C:/ports/Zelda64Recomp/Zelda64Recompiled.exe"),
            g(System::N64, "Super Mario 64", None, "sm64.z64"),
        ] {
            let c = net.candidates(&gr, 12);
            println!("== {}", gr.name);
            for c in &c {
                let ok = net.get_image(&c.url).is_some();
                println!("  {} {} {} {}", if ok { "OK " } else { "-- " }, c.auto_ok, c.label.replace('\t', " | "), c.url);
            }
        }
    }
    #[test]
    fn exe_icon_reads() {
        let p = std::path::Path::new("target/x86_64-pc-windows-gnu/release/nxe-launcher.exe");
        if p.exists() {
            let b = exe_icon(p).expect("icon");
            let i = image::load_from_memory(&b).unwrap();
            println!("icon {}x{}", i.width(), i.height());
            assert!(i.width() >= 64);
        }
    }
}
