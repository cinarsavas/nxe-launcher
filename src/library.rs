use crate::systems::System;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Game {
    pub system: System,
    pub name: String,
    /// What gets passed to the emulator.
    pub path: PathBuf,
    pub kind: &'static str,
    /// Title ID / serial when known.
    pub id: Option<String>,
    /// Encoded image bytes (PNG/JPG) for the tile, if any were found.
    pub cover: Option<Vec<u8>>,
    /// Art that shipped with the game (GOD thumbnail, PS3 ICON0).
    pub embedded: Option<Vec<u8>>,
    /// Cover came from an image file (picked, downloaded, emulator's covers, next to game).
    pub cover_from_file: bool,
}

impl Game {
    pub fn stem(&self) -> String {
        let is_eboot = self.path.file_name().map(|n| n == "EBOOT.BIN").unwrap_or(false);
        let p = if is_eboot {
            // .../<Game Folder>/PS3_GAME/USRDIR/EBOOT.BIN -> <Game Folder>
            self.path.ancestors().find(|a| {
                let n = a.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                !n.is_empty() && n != "EBOOT.BIN" && n != "USRDIR" && n != "PS3_GAME"
            })
        } else {
            Some(self.path.as_path())
        };
        p.and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
    }

    pub fn as_ref(&self) -> crate::covers::GameRef {
        crate::covers::GameRef {
            system: self.system,
            name: self.name.clone(),
            stem: self.stem(),
            id: self.id.clone(),
            path: self.path.clone(),
            embedded: self.embedded.clone(),
        }
    }
}

pub struct CoverDirs {
    /// theme/covers/<console>, theme/covers
    pub manual: Vec<PathBuf>,
    /// emulator cover folders (PCSX2) - searched after manual, before auto
    pub emulator: Vec<PathBuf>,
    /// theme/cache/covers/<console>
    pub auto: PathBuf,
}

pub fn scan(system: System, dirs: &[PathBuf], cover_dirs: &CoverDirs) -> Vec<Game> {
    let mut out = Vec::new();
    if system == System::Pc {
        for d in dirs {
            pc_scan(d, 0, &mut out);
        }
    }
    for d in dirs.iter().filter(|_| system != System::Pc) {
        if d.is_dir() {
            if system == System::Ps3 {
                if let Some(g) = ps3_folder(d) {
                    out.push(g);
                    continue;
                }
            }
            walk(system, d, 0, &mut out);
        }
    }
    let mut seen = HashSet::new();
    out.retain(|g| seen.insert(g.path.to_string_lossy().to_lowercase().replace('\\', "/")));
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    for g in &mut out {
        if system == System::Ps2 && g.id.is_none() {
            g.id = ps2_serial(&g.path);
        }
        g.embedded = g.cover.take();
        resolve_cover(g, cover_dirs);
    }
    out
}

/// Priority: your picks > emulator's own covers / next to the game > auto-downloaded > embedded.
pub fn resolve_cover(g: &mut Game, d: &CoverDirs) {
    let found = find_in(g, &d.manual)
        .or_else(|| find_in(g, &d.emulator))
        .or_else(|| find_beside(g))
        .or_else(|| find_in(g, std::slice::from_ref(&d.auto)));
    g.cover_from_file = found.is_some();
    g.cover = found.or_else(|| g.embedded.clone());
}

fn walk(system: System, dir: &Path, depth: usize, out: &mut Vec<Game>) {
    if depth > 6 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in &entries {
        let fname = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if p.is_dir() {
            let lower = fname.to_lowercase();
            if lower.ends_with(".data") || fname.starts_with('$') {
                continue;
            }
            if system == System::Ps3 {
                if let Some(g) = ps3_folder(p) {
                    out.push(g);
                    continue;
                }
            }
            walk(system, p, depth + 1, out);
            continue;
        }
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        match system {
            System::Pc => {}
            System::Xbox360 => match ext.as_str() {
                "iso" => out.push(simple(system, p, "Disc image")),
                "zar" => out.push(simple(system, p, "ZAR archive")),
                "xex" if fname.eq_ignore_ascii_case("default.xex") => {
                    let name = p
                        .parent()
                        .and_then(|d| d.file_name())
                        .map(|s| clean_name(&s.to_string_lossy()))
                        .unwrap_or_else(|| "Game".into());
                    out.push(Game { system, name, path: p.clone(), kind: "Extracted game", id: None, cover: None, embedded: None, cover_from_file: false });
                }
                "" => {
                    if let Some(g) = read_stfs(p) {
                        out.push(g);
                    }
                }
                _ => {}
            },
            System::Ps3 => {
                if ext == "iso" {
                    out.push(simple(system, p, "Disc image"));
                }
            }
            System::Ps2 => match ext.as_str() {
                "iso" | "chd" | "cso" | "zso" | "gz" | "cue" => out.push(simple(system, p, "Disc image")),
                "bin" => {
                    // Only list a .bin when there's no .cue for it.
                    let cue = p.with_extension("cue");
                    if !cue.is_file() {
                        out.push(simple(system, p, "Disc image"));
                    }
                }
                _ => {}
            },
            System::Switch => match ext.as_str() {
                "nsp" | "nsz" => out.push(simple(system, p, "eShop package")),
                "xci" | "xcz" => out.push(simple(system, p, "Cartridge image")),
                _ => {}
            },
            System::Gc => match ext.as_str() {
                "iso" | "gcm" | "rvz" | "wia" | "gcz" | "ciso" | "wbfs" => {
                    let mut g = simple(system, p, "Disc image");
                    if let Some((id, title)) = gc_header(p) {
                        g.id = Some(id);
                        if !title.is_empty() {
                            g.name = title;
                        }
                    }
                    out.push(g);
                }
                "dol" | "elf" => out.push(simple(system, p, "Homebrew")),
                _ => {}
            },
            System::N64 => match ext.as_str() {
                "z64" | "n64" | "v64" | "ndd" | "zip" => out.push(simple(system, p, "Cartridge")),
                _ => {}
            },
            System::Ds => match ext.as_str() {
                "nds" | "srl" => out.push(simple(system, p, "Game card")),
                _ => {}
            },
            System::Psp => match ext.as_str() {
                "iso" | "cso" | "chd" | "pbp" | "elf" => out.push(simple(system, p, "UMD image")),
                _ => {}
            },
            System::Ps1 => match ext.as_str() {
                "cue" | "chd" | "pbp" | "m3u" | "ecm" | "img" | "iso" => out.push(simple(system, p, "Disc image")),
                "bin" if !p.with_extension("cue").is_file() => out.push(simple(system, p, "Disc image")),
                _ => {}
            },
            System::Xbox => match ext.as_str() {
                "iso" | "xiso" => out.push(simple(system, p, "Disc image")),
                _ => {}
            },
            System::N3ds => match ext.as_str() {
                "3ds" | "cci" | "cxi" => out.push(simple(system, p, "Game card")),
                "3dsx" => out.push(simple(system, p, "Homebrew")),
                _ => {}
            },
        }
    }
}

fn simple(system: System, p: &Path, kind: &'static str) -> Game {
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let id = bracket_id(&stem);
    Game { system, name: clean_name(&stem), path: p.to_path_buf(), kind, id, cover: None, embedded: None, cover_from_file: false }
}

/// Picks up "[0100000000010000]" / "[SLUS-20062]" style IDs from file names.
fn bracket_id(s: &str) -> Option<String> {
    let mut rest = s;
    while let Some(i) = rest.find('[') {
        let after = &rest[i + 1..];
        let Some(j) = after.find(']') else { break };
        let inner = &after[..j];
        let looks_id = inner.len() >= 9
            && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && inner.chars().any(|c| c.is_ascii_digit());
        if looks_id {
            return Some(inner.to_uppercase());
        }
        rest = &after[j + 1..];
    }
    None
}

/// "Halo 3 (USA, Europe) [!]" -> "Halo 3"
pub fn clean_name(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for c in s.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(if c == '_' { ' ' } else { c }),
            _ => {}
        }
    }
    let t = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { s.to_string() } else { t }
}

// ---------------------------------------------------------------- PS3

/// A PS3 game folder: either a disc dump (has PS3_GAME/) or an installed
/// HDD game (has PARAM.SFO + USRDIR/EBOOT.BIN). Updates/DLC folders are skipped.
fn ps3_folder(dir: &Path) -> Option<Game> {
    let (root, kind) = if dir.join("PS3_GAME").join("PARAM.SFO").is_file() {
        (dir.join("PS3_GAME"), "Disc game")
    } else if dir.join("PARAM.SFO").is_file() {
        (dir.to_path_buf(), "Installed game")
    } else {
        return None;
    };
    let eboot = root.join("USRDIR").join("EBOOT.BIN");
    if !eboot.is_file() {
        return None;
    }
    let sfo = std::fs::read(root.join("PARAM.SFO")).ok()?;
    let fields = parse_sfo(&sfo);
    let get = |k: &str| fields.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.clone());
    let cat = get("CATEGORY").unwrap_or_default();
    // DG = disc game, HG = HDD game. GD = patch/update data, others are apps/DLC.
    if kind == "Installed game" && cat != "HG" && cat != "DG" {
        return None;
    }
    let name = get("TITLE")
        .map(|t| t.replace('\n', " "))
        .unwrap_or_else(|| clean_name(&dir.file_name().unwrap_or_default().to_string_lossy()));
    let cover = std::fs::read(root.join("ICON0.PNG")).ok();
    Some(Game { system: System::Ps3, name, path: eboot, kind, id: get("TITLE_ID"), cover, embedded: None, cover_from_file: false })
}

/// PARAM.SFO -> [(key, utf8 value)] (string fields only).
fn parse_sfo(b: &[u8]) -> Vec<(String, String)> {
    let le32 = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as usize);
    let le16 = |o: usize| b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]]) as usize);
    let mut out = Vec::new();
    if b.len() < 20 || &b[0..4] != b"\0PSF" {
        return out;
    }
    let (Some(keys), Some(data), Some(n)) = (le32(8), le32(12), le32(16)) else { return out };
    for i in 0..n.min(256) {
        let e = 20 + i * 16;
        let (Some(ko), Some(fmt), Some(len), Some(doff)) = (le16(e), le16(e + 2), le32(e + 4), le32(e + 12)) else {
            break;
        };
        let kstart = keys + ko;
        let Some(kend) = b.get(kstart..).and_then(|s| s.iter().position(|&c| c == 0)) else { break };
        let key = String::from_utf8_lossy(&b[kstart..kstart + kend]).to_string();
        if fmt == 0x0204 || fmt == 0x0004 {
            if let Some(v) = b.get(data + doff..data + doff + len) {
                let v = String::from_utf8_lossy(v).trim_end_matches('\0').to_string();
                out.push((key, v));
            }
        }
    }
    out
}

// ---------------------------------------------------------------- Xbox 360

fn be32(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Reads an STFS (CON/LIVE/PIRS) header: Games on Demand, Arcade and Demo packages.
/// Takes the title name and the embedded title thumbnail straight from the package.
fn read_stfs(p: &Path) -> Option<Game> {
    let mut f = File::open(p).ok()?;
    let mut buf = vec![0u8; 0xA000];
    let n = f.read(&mut buf).ok()?;
    if n < 0x971A {
        return None;
    }
    let magic = &buf[0..4];
    if magic != b"LIVE" && magic != b"CON " && magic != b"PIRS" {
        return None;
    }
    let kind = match be32(&buf, 0x344) {
        0x7000 => "Games on Demand",
        0xD0000 => "Arcade",
        0x80000 => "Demo",
        _ => return None,
    };
    let title_id = format!("{:08X}", be32(&buf, 0x360));
    let raw = &buf[0x411..0x411 + 0x80];
    let units: Vec<u16> = raw
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    let mut name = String::from_utf16_lossy(&units).trim().to_string();
    if name.is_empty() {
        name = format!("Title {title_id}");
    }
    let tsize = be32(&buf, 0x1716) as usize;
    let psize = be32(&buf, 0x1712) as usize;
    let cover = if tsize > 0 && tsize <= 0x4000 {
        Some(buf[0x571A..0x571A + tsize].to_vec())
    } else if psize > 0 && psize <= 0x4000 {
        Some(buf[0x171A..0x171A + psize].to_vec())
    } else {
        None
    };
    Some(Game { system: System::Xbox360, name, path: p.to_path_buf(), kind, id: Some(title_id), cover, embedded: None, cover_from_file: false })
}

// ---------------------------------------------------------------- covers

const EXTS: [&str; 3] = ["png", "jpg", "jpeg"];

/// <dir>/<ID>.*, <dir>/<Name>.*, <dir>/<file stem>.*
fn find_in(g: &Game, dirs: &[PathBuf]) -> Option<Vec<u8>> {
    let stem = g.stem();
    let mut keys: Vec<String> = Vec::new();
    if let Some(t) = &g.id {
        keys.push(crate::covers::sanitize(t));
    }
    keys.push(crate::covers::sanitize(&g.name));
    keys.push(crate::covers::sanitize(&stem));
    for dir in dirs {
        for k in &keys {
            for e in EXTS {
                let p = dir.join(format!("{k}.{e}"));
                if p.is_file() {
                    return std::fs::read(p).ok();
                }
            }
        }
    }
    None
}

/// <game folder>/<file stem>.* or <game folder>/cover.* (not for PS3 EBOOT paths)
fn find_beside(g: &Game) -> Option<Vec<u8>> {
    if g.system == System::Ps3 && g.path.extension().is_none_or(|e| e != "iso") {
        return None;
    }
    let dir = g.path.parent()?;
    let stem = g.path.file_stem()?.to_string_lossy().to_string();
    for e in EXTS {
        for n in [format!("{stem}.{e}"), format!("cover.{e}")] {
            let p = dir.join(n);
            if p.is_file() {
                return std::fs::read(p).ok();
            }
        }
    }
    None
}

// ---------------------------------------------------------------- GameCube header

/// (game ID like "GALE01", internal title) from a raw .iso/.gcm or an RVZ/WIA header.
fn gc_header(p: &Path) -> Option<(String, String)> {
    let mut f = File::open(p).ok()?;
    let mut b = vec![0u8; 0x100];
    f.read_exact(&mut b).ok()?;
    let off = if &b[0..3] == b"RVZ" || &b[0..3] == b"WIA" { 0x58 } else { 0 };
    let id = &b[off..off + 6];
    if !id.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return None;
    }
    let raw = &b[off + 0x20..off + 0x60];
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    let title = String::from_utf8_lossy(&raw[..end]).trim().to_string();
    Some((String::from_utf8_lossy(id).to_string(), title))
}

// ---------------------------------------------------------------- PS2 serial

/// Reads SYSTEM.CNF from a PS2 .iso (ISO9660) and returns e.g. "SLUS-20062".
fn ps2_serial(p: &Path) -> Option<String> {
    use std::io::{Seek, SeekFrom};
    if p.extension()?.to_string_lossy().to_lowercase() != "iso" {
        return None;
    }
    let mut f = File::open(p).ok()?;
    let mut pvd = [0u8; 2048];
    f.seek(SeekFrom::Start(16 * 2048)).ok()?;
    f.read_exact(&mut pvd).ok()?;
    if &pvd[1..6] != b"CD001" {
        return None;
    }
    let root = &pvd[156..156 + 34];
    let lba = u32::from_le_bytes([root[2], root[3], root[4], root[5]]) as u64;
    let size = (u32::from_le_bytes([root[10], root[11], root[12], root[13]]) as usize).min(64 * 1024);
    let mut dir = vec![0u8; size];
    f.seek(SeekFrom::Start(lba * 2048)).ok()?;
    f.read_exact(&mut dir).ok()?;
    let mut i = 0;
    while i + 33 < dir.len() {
        let len = dir[i] as usize;
        if len == 0 {
            i = (i / 2048 + 1) * 2048;
            continue;
        }
        let nlen = dir[i + 32] as usize;
        let name = String::from_utf8_lossy(&dir[i + 33..(i + 33 + nlen).min(dir.len())]).to_uppercase();
        if name.starts_with("SYSTEM.CNF") {
            let elba = u32::from_le_bytes([dir[i + 2], dir[i + 3], dir[i + 4], dir[i + 5]]) as u64;
            let esz = (u32::from_le_bytes([dir[i + 10], dir[i + 11], dir[i + 12], dir[i + 13]]) as usize).min(4096);
            let mut cnf = vec![0u8; esz];
            f.seek(SeekFrom::Start(elba * 2048)).ok()?;
            f.read_exact(&mut cnf).ok()?;
            let text = String::from_utf8_lossy(&cnf);
            // BOOT2 = cdrom0:\SLUS_200.62;1
            let line = text.lines().find(|l| l.trim_start().starts_with("BOOT2"))?;
            let file = line.rsplit(['\\', ':']).next()?.split(';').next()?.trim();
            let (pre, rest) = file.split_once('_')?;
            let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
            if pre.len() == 4 && digits.len() == 5 {
                return Some(format!("{}-{}", pre.to_uppercase(), digits));
            }
            return None;
        }
        i += len;
    }
    None
}

// ------------------------------------------------------------------ PC ports

/// Words in exe names that are never the game itself.
const NOT_GAME: [&str; 22] = [
    "unins", "uninstall", "setup", "install", "update", "crash", "redist", "vc_", "dxsetup", "dxwebsetup", "helper",
    "dotnet", "createdump", "githublauncher", "n64recomplauncher", "nxe-launcher", "reporter", "elevate", "ue4prereq",
    "vulkan", "7z", "config",
];

fn exes_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false))
        .filter(|p| {
            let n = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            !NOT_GAME.iter().any(|w| n.contains(w))
        })
        .collect();
    v.sort();
    v
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

/// The game's exe in a port folder: the one GitHub Launcher remembers, else the one named most
/// like the folder, else the biggest.
pub fn pc_main_exe(dir: &Path) -> Option<PathBuf> {
    if let Ok(t) = std::fs::read_to_string(dir.join("selected_executable.txt")) {
        let t = t.trim();
        let p = if Path::new(t).is_absolute() { PathBuf::from(t) } else { dir.join(t) };
        if p.is_file() {
            return Some(p);
        }
    }
    let mut exes = exes_in(dir);
    if exes.is_empty() {
        // Some zips unpack into one extra folder (e.g. Port\Port-v1.2\Port.exe).
        let Ok(rd) = std::fs::read_dir(dir) else { return None };
        let subs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        if subs.len() == 1 {
            exes = exes_in(&subs[0]);
        }
    }
    let folder = squash(&dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
    exes.into_iter().max_by_key(|p| {
        let stem = squash(&p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        let named = !stem.is_empty() && (folder.contains(&stem) || stem.contains(&folder));
        let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        (named, size)
    })
}

/// "Zelda64Recompiled" -> "Zelda 64 Recompiled", "UnleashedRecomp" -> "Unleashed Recomp".
pub fn pretty_name(s: &str) -> String {
    // Drop trailing version tags: "Starship_v1.2" / "Port-1.0.3" / "Game Windows x64".
    let is_noise = |t: &str| {
        let l = t.to_lowercase();
        let v = l.strip_prefix('v').unwrap_or(&l);
        (!v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.') && v.chars().next().unwrap().is_ascii_digit() && (l.starts_with('v') || l.contains('.')))
            || matches!(l.as_str(), "win" | "win64" | "windows" | "x64" | "x86" | "portable" | "release")
    };
    let mut toks: Vec<&str> = s.split(['_', '-', ' ']).filter(|t| !t.is_empty()).collect();
    while toks.len() > 1 && is_noise(toks[toks.len() - 1]) {
        toks.pop();
    }
    let s = clean_name(&toks.join(" ").replace('.', " "));
    let mut out = String::new();
    let cs: Vec<char> = s.chars().collect();
    for (i, &c) in cs.iter().enumerate() {
        if i > 0 && c != ' ' && cs[i - 1] != ' ' {
            let prev = cs[i - 1];
            let next_lower = cs.get(i + 1).map(|n| n.is_lowercase()).unwrap_or(false);
            let split = (c.is_uppercase() && (prev.is_lowercase() || (prev.is_uppercase() && next_lower)))
                || (c.is_ascii_digit() && prev.is_lowercase())
                || (c.is_uppercase() && prev.is_ascii_digit() && c != 'D');
            if split {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A folder is either one port (it has the game's exe) or a folder of ports (one per sub-folder).
/// A path straight to an .exe also works.
fn pc_scan(d: &Path, depth: usize, out: &mut Vec<Game>) {
    let game = |exe: PathBuf, name_from: &Path| Game {
        system: System::Pc,
        name: pretty_name(&name_from.file_stem().or(name_from.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
        path: exe,
        kind: "PC port",
        id: None,
        cover: None,
        embedded: None,
        cover_from_file: false,
    };
    if d.is_file() {
        if d.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false) {
            out.push(game(d.to_path_buf(), d));
        }
        return;
    }
    if let Some(exe) = pc_main_exe(d) {
        out.push(game(exe, d));
        return;
    }
    if depth >= 1 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(d) else { return };
    let mut subs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    subs.sort();
    for s in subs {
        pc_scan(&s, depth + 1, out);
    }
}

#[cfg(test)]
mod pc_tests {
    use super::pretty_name;
    #[test]
    fn names() {
        assert_eq!(pretty_name("Zelda64Recompiled"), "Zelda 64 Recompiled");
        assert_eq!(pretty_name("UnleashedRecomp"), "Unleashed Recomp");
        assert_eq!(pretty_name("Sonic3AIR"), "Sonic 3 AIR");
        assert_eq!(pretty_name("SM64CoopDX"), "SM64 Coop DX");
        assert_eq!(pretty_name("Power Moon Recomp"), "Power Moon Recomp");
        assert_eq!(pretty_name("Starship_v1.2"), "Starship");
        assert_eq!(pretty_name("Zelda64Recompiled-v1.2.0-Windows"), "Zelda 64 Recompiled");
    }
}
