//! Your profile file: name, gamerpic, theme, play time per game, pins and hidden games.
//!
//! It lives in Documents\NXE Launcher\profile.json, outside the launcher's folder, so it
//! survives deleting or moving the launcher. When the launcher is set up again, Quick Setup
//! finds it and offers to bring everything back. Games are keyed by console + game ID (or the
//! file name), not by path, so they still match on another drive or PC.

use crate::library::Game;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct GameStat {
    pub name: String,
    pub console: String,
    /// Seconds played through NXE Launcher.
    pub secs: u64,
    pub sessions: u32,
    /// Unix time.
    pub last: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Profile {
    pub format: u32,
    pub gamertag: String,
    pub gamerpic: String,
    pub theme: String,
    pub ra_user: String,
    pub created: i64,
    pub games: HashMap<String, GameStat>,
    pub pinned: Vec<String>,
    pub hidden: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            format: 1,
            gamertag: String::new(),
            gamerpic: String::new(),
            theme: String::new(),
            ra_user: String::new(),
            created: chrono::Utc::now().timestamp(),
            games: HashMap::new(),
            pinned: vec![],
            hidden: vec![],
        }
    }
}

/// Documents\NXE Launcher (falls back to the launcher's folder).
pub fn dir() -> PathBuf {
    if let Ok(d) = std::env::var("NXE_PROFILE_DIR") {
        return PathBuf::from(d);
    }
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|h| PathBuf::from(h).join("Documents").join("NXE Launcher"))
        .unwrap_or_else(|_| crate::config::base_dir().join("profile"))
}

pub fn path() -> PathBuf {
    dir().join("profile.json")
}

/// Copy of your own picture, kept with the profile.
pub fn pic_path() -> PathBuf {
    dir().join("gamerpic.png")
}

pub fn key(g: &Game) -> String {
    let id = g.id.clone().unwrap_or_else(|| crate::library::clean_name(&g.stem()));
    format!("{}|{}", g.system.key(), id.to_lowercase())
}

impl Profile {
    pub fn load_from(p: &std::path::Path) -> Option<Profile> {
        serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()
    }

    pub fn load() -> Profile {
        Self::load_from(&path()).unwrap_or_default()
    }

    pub fn exists() -> bool {
        path().is_file()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(dir());
        if let Ok(t) = serde_json::to_string_pretty(self) {
            // Write then rename, so a crash never leaves half a file.
            let tmp = path().with_extension("json.tmp");
            if std::fs::write(&tmp, t).is_ok() {
                let _ = std::fs::rename(&tmp, path());
            }
        }
    }

    pub fn save_to(&self, p: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(p, serde_json::to_string_pretty(self).unwrap_or_default())
    }

    pub fn stat(&self, g: &Game) -> Option<&GameStat> {
        self.games.get(&key(g))
    }

    pub fn add_session(&mut self, g: &Game, secs: u64) {
        let e = self.games.entry(key(g)).or_default();
        e.name = g.name.clone();
        e.console = g.system.short().to_string();
        e.secs += secs;
        e.sessions += 1;
        e.last = chrono::Utc::now().timestamp();
    }

    pub fn total_secs(&self) -> u64 {
        self.games.values().map(|s| s.secs).sum()
    }

    pub fn is_pinned(&self, g: &Game) -> bool {
        self.pinned.contains(&key(g))
    }

    pub fn is_hidden(&self, g: &Game) -> bool {
        self.hidden.contains(&key(g))
    }

    pub fn toggle_pin(&mut self, g: &Game) -> bool {
        let k = key(g);
        if let Some(i) = self.pinned.iter().position(|p| *p == k) {
            self.pinned.remove(i);
            false
        } else {
            self.pinned.insert(0, k);
            true
        }
    }

    pub fn toggle_hidden(&mut self, g: &Game) -> bool {
        let k = key(g);
        if let Some(i) = self.hidden.iter().position(|p| *p == k) {
            self.hidden.remove(i);
            false
        } else {
            self.hidden.push(k);
            true
        }
    }
}

pub fn fmt_secs(s: u64) -> String {
    let m = s / 60;
    if s == 0 {
        "Not played yet".into()
    } else if m < 1 {
        "Under a minute".into()
    } else if m < 60 {
        format!("{m} min")
    } else {
        format!("{} h {:02} min", m / 60, m % 60)
    }
}

pub fn fmt_date(unix: i64) -> String {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(unix, 0).single() {
        Some(t) => {
            let today = chrono::Local::now().date_naive();
            let d = t.date_naive();
            if d == today {
                format!("Today, {}", t.format("%H:%M"))
            } else if Some(d) == today.pred_opt() {
                format!("Yesterday, {}", t.format("%H:%M"))
            } else {
                t.format("%-d %B %Y").to_string()
            }
        }
        None => "-".into(),
    }
}
