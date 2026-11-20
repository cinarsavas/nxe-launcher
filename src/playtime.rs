//! Play time: what NXE Launcher measures itself, plus what the emulators record on their own.
//!
//! Every format below was checked against the emulator's source code:
//! - PCSX2      `<data>\inis\playtime.dat`            (pcsx2/GameList.cpp)
//! - DuckStation `<data>\playtime.dat`                 (src/core/game_list.cpp, src/core/core.cpp)
//!   both: one line per game, `serial(32) total_seconds(20) last_unix(20)`, space separated
//! - RPCS3      `<config>\GuiConfigs\persistent_settings.dat`, INI `[Playtime] ID=ms`,
//!              `[LastPlayed] ID="MMMM d, yyyy HH:mm"`  (rpcs3qt/persistent_settings.cpp)
//! - Dolphin    `<User>\Config\TimePlayed.ini`, `[TimePlayed] GAMEID=ms` (Core/TimePlayed.cpp)
//! - PPSSPP     `ppsspp.ini`, `[PlayTime] DISCID = seconds,last_unix` (Core/Config.cpp)
//! - yuzu family (Eden, Citron, Sudachi) `<data>\play_time\<user>.bin`, records of
//!              u64 program_id + u64 seconds (yuzu/play_time_manager.cpp)
//! - Ryujinx    `<data>\games\<titleid>\gui\metadata.json`, `timespan_played` / `last_played_utc`
//!              (from Ryujinx's documented format; not re-checked against current Ryubing code)

use crate::library::Game;
use crate::systems::System;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct EmuTime {
    pub secs: u64,
    /// Unix time.
    pub last: Option<i64>,
    pub source: &'static str,
}

fn env_path(v: &str) -> Option<PathBuf> {
    std::env::var(v).ok().filter(|s| !s.is_empty()).map(PathBuf::from)
}

fn documents() -> Option<PathBuf> {
    env_path("USERPROFILE").map(|h| h.join("Documents"))
}

/// Simple INI lookup: value of `key` in `[section]`, quotes removed.
pub fn ini_get(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_sec = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') && l.ends_with(']') {
            in_sec = l[1..l.len() - 1].eq_ignore_ascii_case(section);
            continue;
        }
        if in_sec {
            if let Some((k, v)) = l.split_once('=') {
                if k.trim() == key {
                    return Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// PCSX2 / DuckStation playtime.dat line format.
pub fn parse_playtime_dat(text: &str, serial: &str) -> Option<(u64, i64)> {
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(s), Some(total), Some(last)) = (it.next(), it.next(), it.next()) else { continue };
        if s.eq_ignore_ascii_case(serial) {
            return Some((total.parse().ok()?, last.parse().ok()?));
        }
    }
    None
}

fn exe_dir(exe: Option<&Path>) -> Option<PathBuf> {
    exe.and_then(|e| e.parent()).map(Path::to_path_buf)
}

fn pcsx2(exe: Option<&Path>, serial: &str) -> Option<EmuTime> {
    let mut files = vec![];
    if let Some(d) = exe_dir(exe) {
        // Portable mode: portable.ini / portable.txt next to the exe (portable.txt may name a sub-folder).
        if d.join("portable.ini").is_file() || d.join("portable.txt").is_file() {
            let sub = std::fs::read_to_string(d.join("portable.txt")).unwrap_or_default().trim().to_string();
            files.push(d.join(sub).join("inis").join("playtime.dat"));
        }
        files.push(d.join("inis").join("playtime.dat"));
    }
    if let Some(doc) = documents() {
        files.push(doc.join("PCSX2").join("inis").join("playtime.dat"));
    }
    files.iter().find_map(|f| std::fs::read_to_string(f).ok().and_then(|t| parse_playtime_dat(&t, serial))).map(|(s, l)| EmuTime {
        secs: s,
        last: (l > 0).then_some(l),
        source: "PCSX2",
    })
}

fn duckstation(exe: Option<&Path>, serial: &str) -> Option<EmuTime> {
    let mut files = vec![];
    if let Some(d) = exe_dir(exe) {
        if d.join("portable.txt").is_file() || d.join("settings.ini").is_file() {
            files.push(d.join("playtime.dat"));
        }
    }
    if let Some(doc) = documents() {
        files.push(doc.join("DuckStation").join("playtime.dat"));
    }
    if let Some(l) = env_path("LOCALAPPDATA") {
        files.push(l.join("DuckStation").join("playtime.dat"));
    }
    files.iter().find_map(|f| std::fs::read_to_string(f).ok().and_then(|t| parse_playtime_dat(&t, serial))).map(|(s, l)| EmuTime {
        secs: s,
        last: (l > 0).then_some(l),
        source: "DuckStation",
    })
}

pub fn parse_rpcs3(text: &str, id: &str) -> Option<(u64, Option<i64>)> {
    let ms: u64 = ini_get(text, "Playtime", id)?.parse().ok()?;
    let last = ini_get(text, "LastPlayed", id).and_then(|d| {
        ["%B %d, %Y %H:%M", "%B %d, %Y", "%B %d %Y"].iter().find_map(|f| {
            chrono::NaiveDateTime::parse_from_str(&d, f)
                .ok()
                .or_else(|| chrono::NaiveDate::parse_from_str(&d, f).ok().and_then(|x| x.and_hms_opt(0, 0, 0)))
        })
    });
    Some((ms / 1000, last.and_then(|n| n.and_local_timezone(chrono::Local).single()).map(|t| t.timestamp())))
}

fn rpcs3(exe: Option<&Path>, id: &str) -> Option<EmuTime> {
    let mut dirs = vec![];
    if let Some(d) = exe_dir(exe) {
        dirs.push(d.join("portable"));
    }
    if let Some(c) = env_path("RPCS3_CONFIG_DIR") {
        dirs.push(c);
    }
    if let Some(d) = exe_dir(exe) {
        dirs.push(d);
    }
    dirs.iter().find_map(|d| {
        let t = std::fs::read_to_string(d.join("GuiConfigs").join("persistent_settings.dat")).ok()?;
        parse_rpcs3(&t, id).map(|(secs, last)| EmuTime { secs, last, source: "RPCS3" })
    })
}

fn dolphin(exe: Option<&Path>, id: &str) -> Option<EmuTime> {
    let (_, dolphin_ini) = crate::emusettings::config_file(System::Gc, exe?)?;
    let t = std::fs::read_to_string(dolphin_ini?.with_file_name("TimePlayed.ini")).ok()?;
    let ms: u64 = ini_get(&t, "TimePlayed", id)?.parse().ok()?;
    Some(EmuTime { secs: ms / 1000, last: None, source: "Dolphin" })
}

pub fn parse_ppsspp(text: &str, id: &str) -> Option<(u64, i64)> {
    let v = ini_get(text, "PlayTime", id)?;
    let (a, b) = v.split_once(',')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn ppsspp(exe: Option<&Path>, id: &str) -> Option<EmuTime> {
    let (ini, _) = crate::emusettings::config_file(System::Psp, exe?)?;
    let t = std::fs::read_to_string(ini).ok()?;
    parse_ppsspp(&t, id).map(|(s, l)| EmuTime { secs: s, last: (l > 0).then_some(l), source: "PPSSPP" })
}

/// yuzu-family play_time/*.bin: u64 program id + u64 seconds per record.
pub fn parse_yuzu_bin(b: &[u8], program: u64) -> Option<u64> {
    if b.len() % 16 != 0 {
        return None;
    }
    b.chunks_exact(16).find_map(|r| {
        let id = u64::from_le_bytes(r[..8].try_into().ok()?);
        let secs = u64::from_le_bytes(r[8..].try_into().ok()?);
        (id == program).then_some(secs)
    })
}

/// "[d.]hh:mm:ss[.fffffff]" (.NET TimeSpan "c" format) -> seconds.
pub fn parse_timespan(s: &str) -> Option<u64> {
    let s = s.trim();
    let (days, rest) = match s.split_once('.') {
        Some((d, r)) if !d.contains(':') => (d.parse::<u64>().ok()?, r),
        _ => (0, s),
    };
    let mut parts = rest.split(':');
    let h: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let sec: f64 = parts.next()?.parse().ok()?;
    Some(days * 86400 + h * 3600 + m * 60 + sec as u64)
}

fn switch(exe: Option<&Path>, title_id: &str) -> Option<EmuTime> {
    let program = u64::from_str_radix(title_id, 16).ok()?;
    let base = program & !0xFFF;
    let name = exe.and_then(|e| e.file_stem()).map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let appdata = env_path("APPDATA");
    if name.contains("ryujinx") {
        let mut dirs = vec![];
        if let Some(d) = exe_dir(exe) {
            dirs.push(d.join("portable"));
        }
        if let Some(a) = &appdata {
            dirs.push(a.join("Ryujinx"));
        }
        for d in dirs {
            for id in [format!("{program:016x}"), format!("{base:016x}")] {
                let Ok(t) = std::fs::read_to_string(d.join("games").join(&id).join("gui").join("metadata.json")) else { continue };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) else { continue };
                let secs = v["timespan_played"]
                    .as_str()
                    .and_then(parse_timespan)
                    .or_else(|| v["time_played"].as_f64().map(|f| f as u64))
                    .unwrap_or(0);
                let last = v["last_played_utc"]
                    .as_str()
                    .or(v["last_played"].as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|t| t.timestamp());
                if secs > 0 || last.is_some() {
                    return Some(EmuTime { secs, last, source: "Ryujinx" });
                }
            }
        }
        return None;
    }
    // yuzu family
    let mut dirs = vec![];
    if let Some(d) = exe_dir(exe) {
        dirs.push(d.join("user").join("play_time"));
    }
    if let Some(a) = &appdata {
        for n in ["eden", "citron", "sudachi", "yuzu", "suyu"] {
            if name.is_empty() || name.contains(n) || n == "yuzu" {
                dirs.push(a.join(n).join("play_time"));
            }
        }
    }
    let mut best: Option<u64> = None;
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "bin").unwrap_or(false) {
                if let Ok(b) = std::fs::read(e.path()) {
                    let v = parse_yuzu_bin(&b, program).or_else(|| parse_yuzu_bin(&b, base));
                    if let Some(v) = v {
                        best = Some(best.unwrap_or(0).max(v));
                    }
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    let src = if name.contains("eden") {
        "Eden"
    } else if name.contains("citron") {
        "Citron"
    } else if name.contains("sudachi") {
        "Sudachi"
    } else {
        "yuzu"
    };
    best.map(|secs| EmuTime { secs, last: None, source: src })
}

/// What the game's emulator itself recorded, if it records play time and we know the game's ID.
pub fn emulator_time(g: &Game, exe: Option<&Path>) -> Option<EmuTime> {
    let id = g.id.as_deref()?;
    match g.system {
        System::Ps2 => pcsx2(exe, id),
        System::Ps1 => duckstation(exe, id),
        System::Ps3 => rpcs3(exe, id),
        System::Gc => dolphin(exe, id),
        System::Psp => ppsspp(exe, id),
        System::Switch => switch(exe, id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        // Exactly what PCSX2/DuckStation write: {:<32} {:<20} {:<20}\n
        let line = format!("{:<32} {:<20} {:<20}\n", "SLUS-20946", 7265, 1763600000);
        assert_eq!(parse_playtime_dat(&line, "SLUS-20946"), Some((7265, 1763600000)));
        let rp = "[LastPlayed]\nBLUS30443=\"November 20, 2026 21:30\"\n\n[Playtime]\nBLUS30443=5400000\n";
        let (s, l) = parse_rpcs3(rp, "BLUS30443").unwrap();
        assert_eq!(s, 5400);
        assert!(l.is_some());
        assert_eq!(ini_get("[TimePlayed]\nGALE01 = 3600000\n", "TimePlayed", "GALE01").as_deref(), Some("3600000"));
        assert_eq!(parse_ppsspp("[PlayTime]\nULUS10336 = 4000,1763600000\n", "ULUS10336"), Some((4000, 1763600000)));
        let mut b = vec![];
        b.extend_from_slice(&0x0100_0000_0010_0000u64.to_le_bytes());
        b.extend_from_slice(&999u64.to_le_bytes());
        assert_eq!(parse_yuzu_bin(&b, 0x0100_0000_0010_0000), Some(999));
        assert_eq!(parse_timespan("01:02:03.5000000"), Some(3723));
        assert_eq!(parse_timespan("2.01:00:00"), Some(2 * 86400 + 3600));
    }
}
