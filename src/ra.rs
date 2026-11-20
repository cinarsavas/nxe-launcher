//! RetroAchievements through the same sign-in emulators use: username +
//! password once, then only the session token is kept (the password is never
//! saved). Shows your points/picture, per-game progress and the achievement list.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

const CONNECT: &str = "https://retroachievements.org/dorequest.php";
const MEDIA: &str = "https://media.retroachievements.org";
/// RetroAchievements console IDs we care about: PS2, GameCube, N64, DS, PSP, PS1.
const CONSOLES: [u64; 6] = [21, 16, 2, 18, 41, 12];

#[derive(Clone, Debug)]
pub struct Achievement {
    pub title: String,
    pub desc: String,
    pub points: u32,
    pub badge: String,
    pub earned: bool,
    pub hardcore: bool,
}

pub enum Job {
    /// Sign in with a password (first time) — returns a token.
    Login { user: String, password: String },
    /// Refresh profile + game lists using the saved token.
    Refresh { user: String, token: String },
    /// Achievements + your unlocks for one game. `req == 0` = progress only (no badges).
    Game { req: u64, game_id: u64, user: String, token: String },
}

pub enum Msg {
    LoggedIn { user: String, token: String },
    Profile { points: u64, pic: Option<Vec<u8>> },
    /// (console, game id, title) for every supported console.
    Library { sets: Vec<(u64, u64, String)> },
    Game { req: u64, game_id: u64, title: String, list: Vec<Achievement> },
    Badge { req: u64, idx: usize, bytes: Vec<u8> },
    Error(String),
}

pub fn spawn(cache: PathBuf, rx: Receiver<Job>, tx: Sender<Msg>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(25)))
            .http_status_as_error(false)
            .user_agent(concat!("NXELauncher/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        let get = |url: &str| -> Result<Vec<u8>, String> {
            let mut r = agent.get(url).call().map_err(|e| e.to_string())?;
            if !r.status().is_success() {
                return Err(format!("HTTP {}", r.status()));
            }
            r.body_mut().with_config().limit(40 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())
        };
        let post = |form: &[(&str, &str)]| -> Result<serde_json::Value, String> {
            let mut r = agent.post(CONNECT).send_form(form.iter().copied()).map_err(|e| e.to_string())?;
            let v: serde_json::Value = serde_json::from_slice(&r.body_mut().read_to_vec().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            if v["Success"].as_bool() == Some(false) {
                return Err(v["Error"].as_str().unwrap_or("request failed").to_string());
            }
            Ok(v)
        };
        let profile = |v: &serde_json::Value| -> Msg {
            let points = v["Score"].as_u64().unwrap_or(0) + v["SoftcoreScore"].as_u64().unwrap_or(0);
            let pic = v["AvatarUrl"].as_str().and_then(|u| get(u).ok());
            Msg::Profile { points, pic }
        };
        while let Ok(job) = rx.recv() {
            match job {
                Job::Login { user, password } => match post(&[("r", "login2"), ("u", &user), ("p", &password)]) {
                    Ok(v) => {
                        let token = v["Token"].as_str().unwrap_or("").to_string();
                        let user = v["User"].as_str().unwrap_or(&user).to_string();
                        let _ = tx.send(Msg::LoggedIn { user: user.clone(), token: token.clone() });
                        let _ = tx.send(profile(&v));
                        let _ = tx.send(Msg::Library { sets: library(&cache, &post) });
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::Error(format!("Sign-in failed: {e}")));
                    }
                },
                Job::Refresh { user, token } => match post(&[("r", "login2"), ("u", &user), ("t", &token)]) {
                    Ok(v) => {
                        let _ = tx.send(profile(&v));
                        let _ = tx.send(Msg::Library { sets: library(&cache, &post) });
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::Error(format!("RetroAchievements: {e}. Sign in again in Settings.")));
                    }
                },
                Job::Game { req, game_id, user, token } => {
                    let g = game_id.to_string();
                    let patch = post(&[("r", "patch"), ("u", &user), ("t", &token), ("g", &g)]);
                    let soft = post(&[("r", "unlocks"), ("u", &user), ("t", &token), ("g", &g), ("h", "0")]);
                    let hard = post(&[("r", "unlocks"), ("u", &user), ("t", &token), ("g", &g), ("h", "1")]);
                    let ids = |v: &Result<serde_json::Value, String>| -> Vec<u64> {
                        v.as_ref().ok().and_then(|v| v["UserUnlocks"].as_array().cloned()).unwrap_or_default().iter().filter_map(|x| x.as_u64()).collect()
                    };
                    let (soft, hard) = (ids(&soft), ids(&hard));
                    match patch {
                        Ok(v) => {
                            let pd = &v["PatchData"];
                            // Flags 3 = official achievements (5 = unofficial/test).
                            let list: Vec<Achievement> = pd["Achievements"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter(|a| a["Flags"].as_u64().unwrap_or(3) == 3)
                                .map(|a| {
                                    let id = a["ID"].as_u64().unwrap_or(0);
                                    let hc = hard.contains(&id);
                                    Achievement {
                                        title: a["Title"].as_str().unwrap_or("").to_string(),
                                        desc: a["Description"].as_str().unwrap_or("").to_string(),
                                        points: a["Points"].as_u64().unwrap_or(0) as u32,
                                        badge: a["BadgeName"].as_str().unwrap_or("").to_string(),
                                        earned: hc || soft.contains(&id),
                                        hardcore: hc,
                                    }
                                })
                                .collect();
                            let title = pd["Title"].as_str().unwrap_or("").to_string();
                            let badges: Vec<(String, bool)> = list.iter().map(|a| (a.badge.clone(), a.earned)).collect();
                            let _ = tx.send(Msg::Game { req, game_id, title, list });
                            ctx.request_repaint();
                            if req != 0 {
                                for (idx, (b, earned)) in badges.into_iter().enumerate() {
                                    if b.is_empty() {
                                        continue;
                                    }
                                    let suffix = if earned { "" } else { "_lock" };
                                    if let Ok(bytes) = get(&format!("{MEDIA}/Badge/{b}{suffix}.png")) {
                                        let _ = tx.send(Msg::Badge { req, idx, bytes });
                                        ctx.request_repaint();
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if req != 0 {
                                let _ = tx.send(Msg::Error(format!("Couldn't load achievements: {e}")));
                            }
                        }
                    }
                }
            }
            ctx.request_repaint();
        }
    });
}

/// Game titles per console (cached for a week).
fn library(cache: &std::path::Path, post: &dyn Fn(&[(&str, &str)]) -> Result<serde_json::Value, String>) -> Vec<(u64, u64, String)> {
    let mut sets = Vec::new();
    for console in CONSOLES {
        let file = cache.join(format!("ra-games-{console}.json"));
        let fresh = std::fs::metadata(&file)
            .and_then(|m| m.modified())
            .map(|t| t.elapsed().unwrap_or_default() < Duration::from_secs(7 * 86400))
            .unwrap_or(false);
        let v: Option<serde_json::Value> = if fresh { std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()) } else { None }.or_else(|| {
            let c = console.to_string();
            let v = post(&[("r", "gameslist"), ("c", &c)]).ok()?;
            let _ = std::fs::create_dir_all(cache);
            let _ = std::fs::write(&file, v.to_string());
            Some(v)
        });
        if let Some(map) = v.as_ref().and_then(|v| v["Response"].as_object()) {
            for (id, t) in map {
                if let (Ok(id), Some(t)) = (id.parse::<u64>(), t.as_str()) {
                    sets.push((console, id, t.to_string()));
                }
            }
        }
    }
    sets
}

/// Best RA game for a local game on the given RA console.
pub fn match_set(console: u64, name: &str, stem: &str, sets: &[(u64, u64, String)]) -> Option<u64> {
    let mine: Vec<&(u64, u64, String)> = sets.iter().filter(|s| s.0 == console && !s.2.starts_with('~')).collect();
    let titles: Vec<String> = mine.iter().map(|s| s.2.clone()).collect();
    let best = crate::covers::best_matches_pub(name, stem, &titles, 1);
    let (t, score) = best.into_iter().next()?;
    if score < 0.75 {
        return None;
    }
    mine.iter().find(|s| s.2 == t).map(|s| s.1)
}

/// Is the user signed in to RetroAchievements inside PCSX2?
pub fn pcsx2_status(exe: Option<&std::path::Path>) -> Option<(bool, String)> {
    let mut inis = Vec::new();
    if let Some(d) = exe.and_then(|e| e.parent()) {
        inis.push(d.join("inis").join("PCSX2.ini"));
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        inis.push(PathBuf::from(h).join("Documents").join("PCSX2").join("inis").join("PCSX2.ini"));
    }
    for ini in inis {
        if let Ok(s) = std::fs::read_to_string(&ini) {
            let mut sec = false;
            let mut enabled = false;
            let mut user = String::new();
            for l in s.lines() {
                let l = l.trim();
                if l.starts_with('[') {
                    sec = l.eq_ignore_ascii_case("[Achievements]");
                    continue;
                }
                if sec {
                    if let Some((k, v)) = l.split_once('=') {
                        match k.trim() {
                            "Enabled" => enabled = v.trim().eq_ignore_ascii_case("true"),
                            "Username" => user = v.trim().to_string(),
                            _ => {}
                        }
                    }
                }
            }
            return Some((enabled && !user.is_empty(), user));
        }
    }
    None
}

#[allow(dead_code)]
pub type Progress = HashMap<u64, (u32, u32)>;
