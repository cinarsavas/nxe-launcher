#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod config;
mod covers;
mod emusettings;
mod input;
mod library;
mod menu;
mod profile;
mod ra;
mod runtime;
mod scene;
mod sound;
mod systems;
mod winapi;

use config::{base_dir, Config};
use eframe::egui;
use input::{Pad, Shared};
use library::Game;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use systems::System;

#[derive(Clone, Debug)]
pub enum Action {
    Launch(usize),
    GoToChannel(usize),
    AddFolder(System),
    OpenSettings,
    OpenSetup,
    Rescan,
    Exit,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Play,
    Plus,
    Gear,
    Refresh,
    Doc,
    Expand,
    Cross,
    Power,
    Grid,
    Trophy,
    Disc,
    Palette,
    Image,
    Info,
}

#[derive(Clone)]
pub struct Tile {
    pub title: String,
    pub subtitle: String,
    pub action: Action,
    pub game: Option<usize>,
    /// Game whose cover is used as art (defaults to `game`).
    pub art: Option<usize>,
    pub glyph: Icon,
    /// Tile colour; None = theme colour.
    pub accent: Option<egui::Color32>,
}

pub struct Channel {
    pub name: String,
    pub tiles: Vec<Tile>,
    pub sel: usize,
    pub anim: f32,
    pub vel: f32,
}

pub struct App {
    pub cfg: Config,
    pub games: Vec<Game>,
    covers: HashMap<usize, Option<(egui::TextureHandle, egui::Color32)>>,
    background: Option<egui::TextureHandle>,
    channels: Vec<Channel>,
    chan: usize,
    /// Unbounded channel position (wraps around); chan = chan_pos mod n.
    chan_pos: f32,
    chan_anim: f32,
    rx: Receiver<Pad>,
    shared: Arc<Shared>,
    pub sounds: sound::Sounds,
    launching: Option<(Instant, Option<usize>, Option<System>)>,
    playing: Option<String>,
    returned: Option<Instant>,
    toast: Option<(String, Instant)>,
    pub fullscreen: bool,
    last_frame: Instant,
    start: Instant,
    pub menu: Option<menu::Menu>,
    pub picker: Option<menu::Picker>,
    pub picker_req: u64,
    cover_auto_tx: Sender<covers::Job>,
    pub cover_pri_tx: Sender<covers::Job>,
    cover_rx: Receiver<covers::Msg>,
    pub cover_progress: (usize, usize),
    pub tried: HashSet<String>,
    last_pointer: (egui::Pos2, Instant),
    ra_tx: Sender<ra::Job>,
    ra_rx: Receiver<ra::Msg>,
    pub ra_points_val: Option<u64>,
    ra_avatar_tex: Option<egui::TextureHandle>,
    pub ra_sets: Vec<(u64, u64, String)>,
    pub ra_progress: HashMap<u64, (u32, u32)>,
    /// game index -> (RA game id, achievement count)
    pub ra_match: HashMap<usize, u64>,
    ra_requested: HashSet<u64>,
    pub ra_status: Option<String>,
    pub ach: Option<menu::AchView>,
    rt_tx: Sender<runtime::Job>,
    rt_rx: Receiver<runtime::Msg>,
    /// Built-in core being prepared: (console, game to start afterwards, stage, progress, error).
    pub installing: Option<Installing>,
    /// When the focused tile last changed (for the selection "pop").
    pub sel_changed: Instant,
    chan_vel: f32,
    pub dialog: Option<menu::Dialog>,
    /// Dashboard intro animation start (app start, or after quick setup).
    intro: Option<Instant>,
    /// Theme colour cross-fade.
    theme_from: scene::Theme,
    theme_changed: Instant,
    pub pic_picker: Option<profile::PicPicker>,
    pub cropper: Option<profile::Cropper>,
    custom_pic_tex: Option<egui::TextureHandle>,
    /// Typed password, kept only in memory until you press Sign In.
    pub ra_password: String,
    page_anim: f32,
    page_vel: f32,
    sub_anim: f32,
    sub_vel: f32,
    sub_from: f32,
    last_top: Option<String>,
    last_page: Option<String>,
    pub user_pics: Vec<(String, egui::TextureHandle)>,
    /// Picked the RetroAchievements picture before signing in: use it once sign-in works.
    pub ra_pic_pending: bool,
    last_paint: Instant,
    ctx_handle: Option<egui::Context>,
}

pub struct Installing {
    pub system: System,
    pub game: Option<usize>,
    pub stage: String,
    pub frac: f32,
    pub shown: f32,
    pub error: Option<String>,
    pub visible: bool,
    pub started: Instant,
}

pub fn theme_dir() -> PathBuf {
    base_dir().join("theme")
}


impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = Config::load();
        install_font(&cc.egui_ctx);
        let (tx, rx) = channel();
        let shared = Arc::new(Shared { child: Mutex::new(None), running: false.into(), closing: false.into() });
        input::spawn(tx, cc.egui_ctx.clone(), shared.clone());
        let (auto_tx, auto_rx) = channel();
        let (pri_tx, pri_rx) = channel();
        let (msg_tx, msg_rx) = channel();
        covers::spawn(theme_dir(), auto_rx, pri_rx, msg_tx, cc.egui_ctx.clone());
        let (ra_tx, ra_jobs) = channel();
        let (ra_msg_tx, ra_rx) = channel();
        ra::spawn(theme_dir().join("cache"), ra_jobs, ra_msg_tx, cc.egui_ctx.clone());
        let (rt_tx, rt_jobs) = channel();
        let (rt_msg_tx, rt_rx) = channel();
        runtime::spawn(rt_jobs, rt_msg_tx, cc.egui_ctx.clone());
        let background = load_texture(&cc.egui_ctx, "bg", &theme_dir().join("background.png"))
            .or_else(|| load_texture(&cc.egui_ctx, "bg", &theme_dir().join("background.jpg")));
        let sounds = sound::Sounds::new(theme_dir().join("sounds"), cfg.sounds);
        let fullscreen = cfg.fullscreen;
        let setup = !cfg.setup_done;
        let mut app = Self {
            cfg,
            games: vec![],
            covers: HashMap::new(),
            background,
            channels: vec![],
            chan: 0,
            chan_pos: 0.0,
            chan_anim: 0.0,
            rx,
            shared,
            sounds,
            launching: None,
            playing: None,
            returned: None,
            toast: None,
            fullscreen,
            last_frame: Instant::now(),
            start: Instant::now(),
            menu: if setup { Some(menu::Menu::new(menu::Kind::Setup)) } else { None },
            picker: None,
            picker_req: 0,
            cover_auto_tx: auto_tx,
            cover_pri_tx: pri_tx,
            cover_rx: msg_rx,
            cover_progress: (0, 0),
            tried: covers::load_tried(&theme_dir()),
            last_pointer: (egui::Pos2::ZERO, Instant::now()),
            ra_tx,
            ra_rx,
            ra_points_val: None,
            ra_avatar_tex: None,
            ra_sets: vec![],
            ra_progress: HashMap::new(),
            ra_match: HashMap::new(),
            ra_requested: HashSet::new(),
            ra_status: None,
            ach: None,
            rt_tx,
            rt_rx,
            installing: None,
            sel_changed: Instant::now(),
            chan_vel: 0.0,
            dialog: None,
            intro: None,
            theme_from: scene::theme("green"),
            theme_changed: Instant::now() - Duration::from_secs(5),
            pic_picker: None,
            cropper: None,
            custom_pic_tex: None,
            ra_password: String::new(),
            page_anim: 0.0,
            page_vel: 0.0,
            sub_anim: 1.0,
            sub_vel: 0.0,
            sub_from: 1.0,
            last_top: None,
            last_page: None,
            user_pics: vec![],
            ra_pic_pending: false,
            last_paint: Instant::now(),
            ctx_handle: Some(cc.egui_ctx.clone()),
        };
        app.reload_custom_pic_ctx(&cc.egui_ctx);
        app.user_pics = profile::load_user_pics(&cc.egui_ctx);
        app.load_cached_ra_avatar(&cc.egui_ctx);
        app.theme_from = scene::theme(&app.cfg.theme);
        if !setup {
            app.intro = Some(Instant::now());
        }
        app.rescan();
        app.ra_refresh();
        if std::env::var("NXE_FAKE_RA").is_ok() {
            app.ra_sets = vec![(21, 1, "Shadow of the Colossus".into()), (21, 2, "Grand Theft Auto: San Andreas".into())];
            app.ra_progress.insert(1, (12, 40));
            app.ra_requested.insert(1);
            app.ra_points_val = Some(1234);
            app.cfg.ra_user = "test".into();
            app.cfg.ra_token = "test".into();
            app.ra_rematch();
        }
        app
    }

    fn cover_dirs(&self, s: System) -> library::CoverDirs {
        let mut d = menu::theme_cover_dirs(s);
        d.emulator = s.emulator_cover_dirs(self.cfg.emulator(s).as_deref());
        d
    }

    pub fn rescan(&mut self) {
        let mut all = Vec::new();
        for s in systems::ALL {
            let c = self.cfg.sys(s);
            let emu = self.cfg.emulator(s);
            let mut dirs: Vec<PathBuf> = c.game_dirs.iter().map(PathBuf::from).collect();
            if c.import_from_emulator {
                if let Some(e) = &emu {
                    dirs.extend(s.imported_dirs(e));
                }
            }
            all.extend(library::scan(s, &dirs, &self.cover_dirs(s)));
        }
        self.games = all;
        self.covers.clear();
        self.ra_rematch();
        self.rebuild();
        if self.cfg.auto_covers {
            self.queue_auto_covers();
        }
    }

    fn wants_online(g: &Game) -> bool {
        // PS3 ICON0 is official art already; a 64px Xbox GOD thumbnail is worth replacing.
        !g.cover_from_file && (g.embedded.is_none() || g.system == System::Xbox360)
    }

    pub fn queue_auto_covers(&mut self) {
        for g in &self.games {
            if Self::wants_online(g) && !self.tried.contains(&covers::tried_key(&g.as_ref())) {
                let _ = self.cover_auto_tx.send(covers::Job::Auto(g.as_ref()));
            }
        }
    }

    pub fn queue_auto_covers_forced(&mut self) {
        for g in &self.games {
            if Self::wants_online(g) {
                let _ = self.cover_auto_tx.send(covers::Job::Auto(g.as_ref()));
            }
        }
    }

    pub fn refresh_cover(&mut self, gi: usize) {
        let s = self.games[gi].system;
        let d = self.cover_dirs(s);
        library::resolve_cover(&mut self.games[gi], &d);
        self.covers.remove(&gi);
    }

    fn poll_covers(&mut self, ctx: &egui::Context) {
        let msgs: Vec<covers::Msg> = self.cover_rx.try_iter().collect();
        for m in msgs {
            match m {
                covers::Msg::Saved(path) => {
                    if let Some(i) = self.games.iter().position(|g| g.path == path) {
                        self.refresh_cover(i);
                    }
                }
                covers::Msg::Candidates(req, list) => self.picker_candidates(ctx, req, list),
                covers::Msg::CandidatesDone(req) => {
                    if let Some(p) = self.picker.as_mut().filter(|p| p.req == req) {
                        p.done = true;
                    }
                }
                covers::Msg::Progress { done, queued } => self.cover_progress = (done, queued),
            }
        }
    }

    fn rebuild(&mut self) {
        let keep: HashMap<String, usize> = self.channels.iter().map(|c| (c.name.clone(), c.sel)).collect();
        let cur_name = self.channels.get(self.chan).map(|c| c.name.clone());

        let mut named: Vec<(String, Vec<Tile>)> = Vec::new();

        // ---- home: recently played across every console
        let mut home = Vec::new();
        let recent: Vec<usize> = self
            .cfg
            .recent
            .iter()
            .filter_map(|p| self.games.iter().position(|g| g.path.to_string_lossy() == *p))
            .take(6)
            .collect();
        for (i, gi) in recent.iter().enumerate() {
            let g = &self.games[*gi];
            home.push(Tile {
                title: g.name.clone(),
                subtitle: format!("{} · {}", if i == 0 { "Continue playing" } else { "Recently played" }, g.system.short()),
                action: Action::Launch(*gi),
                game: Some(*gi),
                art: None,
                glyph: Icon::Play,
                accent: None,
            });
        }

        // ---- one channel per console that's actually set up
        let mut console_chans = Vec::new();
        for s in systems::ALL {
            let emu = self.cfg.emulator(s);
            let games: Vec<(usize, &Game)> = self.games.iter().enumerate().filter(|(_, g)| g.system == s).collect();
            if emu.is_none() && games.is_empty() {
                continue;
            }
            let mut tiles: Vec<Tile> = games
                .iter()
                .map(|(i, g)| Tile {
                    title: g.name.clone(),
                    subtitle: match &g.id {
                        Some(t) => format!("{} · {}", g.kind, t),
                        None => g.kind.to_string(),
                    },
                    action: Action::Launch(*i),
                    game: Some(*i),
                    art: None,
                    glyph: Icon::Play,
                    accent: None,
                })
                .collect();
            if tiles.is_empty() {
                tiles.push(Tile {
                    title: "Add games".into(),
                    subtitle: format!("No {} games yet. Pick a folder with {}", s.short(), s.folder_hint()),
                    action: Action::AddFolder(s),
                    game: None,
                    art: None,
                    glyph: Icon::Plus,
                    accent: None,
                });
            }
            console_chans.push((s.channel().to_string(), tiles));
        }

        if recent.is_empty() {
            for (ci, (name, tiles)) in console_chans.iter().enumerate() {
                let n = tiles.iter().filter(|t| t.game.is_some()).count();
                if n > 0 {
                    home.push(Tile {
                        title: name.clone(),
                        subtitle: format!("{n} game(s)"),
                        action: Action::GoToChannel(ci + 1),
                        game: None,
                        art: tiles.iter().filter_map(|t| t.game).find(|g| self.games[*g].cover.is_some()),
                        glyph: Icon::Grid,
                        accent: None,
                    });
                }
            }
        }
        if self.games.is_empty() {
            home.push(Tile {
                title: "Quick Setup".into(),
                subtitle: "Find your emulators and games".into(),
                action: Action::OpenSetup,
                game: None,
                art: None,
                glyph: Icon::Gear,
                accent: None,
            });
        }
        named.push(("Home".into(), home));
        named.extend(console_chans);

        // ---- system
        named.push((
            "System".into(),
            vec![
                Tile {
                    title: "Settings".into(),
                    subtitle: "Emulators, game folders, covers, display".into(),
                    action: Action::OpenSettings,
                    game: None,
                    art: None,
                    glyph: Icon::Gear,
                    accent: None,
                },
                Tile {
                    title: "Rescan library".into(),
                    subtitle: format!("{} games across all consoles", self.games.len()),
                    action: Action::Rescan,
                    game: None,
                    art: None,
                    glyph: Icon::Refresh,
                    accent: None,
                },
                Tile {
                    title: "Exit".into(),
                    subtitle: "Close the dashboard".into(),
                    action: Action::Exit,
                    game: None,
                    art: None,
                    glyph: Icon::Power,
                    accent: None,
                },
            ],
        ));

        self.channels = named
            .into_iter()
            .map(|(name, tiles)| {
                let sel = keep.get(&name).copied().unwrap_or(0).min(tiles.len().saturating_sub(1));
                Channel { name, tiles, sel, anim: sel as f32, vel: 0.0 }
            })
            .collect();
        let c = cur_name
            .and_then(|n| self.channels.iter().position(|c| c.name == n))
            .unwrap_or(0)
            .min(self.channels.len() - 1);
        self.set_chan(c);
    }

    pub fn theme_now(&self) -> scene::Theme {
        let to = scene::theme(&self.cfg.theme);
        let t = scene::seg(self.theme_changed.elapsed().as_secs_f32(), 0.0, 0.7);
        scene::lerp_theme(&self.theme_from, &to, t)
    }

    pub fn set_theme(&mut self, name: &str) {
        self.theme_from = self.theme_now();
        self.theme_changed = Instant::now();
        self.cfg.theme = name.to_string();
        self.cfg.save();
    }

    pub fn intro_t(&self) -> Option<f32> {
        self.intro.map(|t| t.elapsed().as_secs_f32()).filter(|t| *t < 2.2)
    }

    pub fn ctx(&self) -> Option<&egui::Context> {
        self.ctx_handle.as_ref()
    }

    pub fn custom_pic(&self) -> Option<egui::TextureId> {
        self.custom_pic_tex.as_ref().map(|t| t.id())
    }

    fn load_cached_ra_avatar(&mut self, ctx: &egui::Context) {
        if self.ra_signed_in() {
            self.ra_avatar_tex = std::fs::read(theme_dir().join("cache").join("ra_avatar.png")).ok().and_then(|b| load_tex(ctx, "ra_avatar", &b));
        }
    }

    fn reload_custom_pic_ctx(&mut self, ctx: &egui::Context) {
        self.custom_pic_tex = std::fs::read(theme_dir().join("profile.png")).ok().and_then(|b| {
            let img = image::load_from_memory(&b).ok()?.to_rgba8();
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            Some(ctx.load_texture("custom_pic", ci, egui::TextureOptions::LINEAR))
        });
    }

    pub fn reload_custom_pic(&mut self) {
        if let Some(ctx) = self.ctx_handle.clone() {
            self.reload_custom_pic_ctx(&ctx);
        }
    }

    pub fn start_intro(&mut self) {
        self.intro = Some(Instant::now());
        self.sel_changed = Instant::now();
    }

    pub fn close_menu(&mut self) {
        if let Some(m) = self.menu.as_mut() {
            if m.closing.is_none() {
                m.closing = Some(Instant::now());
                m.edit = None;
            }
        }
    }

    pub fn set_chan(&mut self, c: usize) {
        let n = self.channels.len().max(1) as f32;
        let cur = self.chan_pos.rem_euclid(n);
        let mut d = c as f32 - cur;
        if d > n / 2.0 {
            d -= n;
        } else if d < -n / 2.0 {
            d += n;
        }
        self.chan_pos += d;
        self.chan = c;
    }

    fn step_chan(&mut self, d: i32) {
        let n = self.channels.len().max(1) as i32;
        self.chan_pos += d as f32;
        self.chan = (self.chan as i32 + d).rem_euclid(n) as usize;
    }

    // ---- RetroAchievements
    pub fn ra_signed_in(&self) -> bool {
        !self.cfg.ra_user.trim().is_empty() && !self.cfg.ra_token.trim().is_empty()
    }

    pub fn ra_refresh(&mut self) {
        if !self.ra_signed_in() {
            self.ra_points_val = None;
            self.ra_avatar_tex = None;
            self.ra_match.clear();
            return;
        }
        self.ra_status = Some("Connecting…".into());
        let _ = self.ra_tx.send(ra::Job::Refresh { user: self.cfg.ra_user.trim().into(), token: self.cfg.ra_token.trim().into() });
    }

    pub fn ra_login(&mut self, password: String) {
        if self.cfg.ra_user.trim().is_empty() {
            self.toast("Enter your username first");
            return;
        }
        self.ra_status = Some("Signing in…".into());
        let _ = self.ra_tx.send(ra::Job::Login { user: self.cfg.ra_user.trim().into(), password });
    }

    pub fn ra_logout(&mut self) {
        let _ = std::fs::remove_file(theme_dir().join("cache").join("ra_avatar.png"));
        if self.cfg.gamerpic == "ra" {
            self.cfg.gamerpic = "builtin:0".into();
        }
        self.cfg.ra_token.clear();
        self.cfg.save();
        self.ra_points_val = None;
        self.ra_avatar_tex = None;
        self.ra_match.clear();
        self.ra_progress.clear();
        self.ra_requested.clear();
        self.ra_status = Some("Signed out".into());
    }

    /// Fetch progress for the focused game once it has been focused for a moment.
    fn ra_prefetch(&mut self) {
        if !self.ra_signed_in() || self.sel_changed.elapsed().as_secs_f32() < 0.35 {
            return;
        }
        let Some(gi) = self.focused_game() else { return };
        let Some(&id) = self.ra_match.get(&gi) else { return };
        if self.ra_requested.insert(id) {
            let _ = self.ra_tx.send(ra::Job::Game { req: 0, game_id: id, user: self.cfg.ra_user.clone(), token: self.cfg.ra_token.clone() });
        }
    }

    fn ra_rematch(&mut self) {
        self.ra_match.clear();
        if self.ra_sets.is_empty() {
            return;
        }
        for (i, g) in self.games.iter().enumerate() {
            if let Some(console) = g.system.ra_console() {
                if let Some(m) = ra::match_set(console, &g.name, &g.stem(), &self.ra_sets) {
                    self.ra_match.insert(i, m);
                }
            }
        }
    }

    fn poll_ra(&mut self, ctx: &egui::Context) {
        let msgs: Vec<ra::Msg> = self.ra_rx.try_iter().collect();
        for m in msgs {
            match m {
                ra::Msg::LoggedIn { user, token } => {
                    self.cfg.ra_user = user.clone();
                    self.cfg.ra_token = token;
                    self.cfg.save();
                    self.toast(format!("Signed in as {user}"));
                }
                ra::Msg::Profile { points, pic } => {
                    self.ra_points_val = Some(points);
                    self.ra_status = Some(format!("Signed in as {} · {points} points", self.cfg.ra_user));
                    if let Some(b) = &pic {
                        // Keep a copy so the picture also shows offline and right at startup.
                        if image::load_from_memory(b).is_ok() {
                            let _ = std::fs::write(theme_dir().join("cache").join("ra_avatar.png"), b);
                        }
                    }
                    if let Some(t) = pic.and_then(|b| load_tex(ctx, "ra_avatar", &b)) {
                        self.ra_avatar_tex = Some(t);
                    }
                    if self.ra_pic_pending {
                        self.ra_pic_pending = false;
                        if self.ra_avatar_tex.is_some() {
                            self.cfg.gamerpic = "ra".into();
                            self.cfg.save();
                            self.toast("Gamerpic: your RetroAchievements picture");
                        }
                    }
                }
                ra::Msg::Library { sets } => {
                    self.ra_sets = sets;
                    self.ra_rematch();
                }
                ra::Msg::Game { req, game_id, title, list } => {
                    let earned = list.iter().filter(|a| a.earned).count() as u32;
                    self.ra_progress.insert(game_id, (earned, list.len() as u32));
                    if req != 0 {
                        self.ach_loaded(ctx, req, title, list);
                    }
                }
                ra::Msg::Badge { req, idx, bytes } => self.ach_badge(ctx, req, idx, bytes),
                ra::Msg::Error(e) => {
                    self.ra_status = Some(e.clone());
                    self.toast(e);
                }
            }
        }
    }

    pub fn ra_tx_send(&self, j: ra::Job) -> Result<(), std::sync::mpsc::SendError<ra::Job>> {
        self.ra_tx.send(j)
    }

    pub fn toast(&mut self, s: impl Into<String>) {
        self.toast = Some((s.into(), Instant::now()));
    }

    fn activate(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        let Some(tile) = self.channels.get(self.chan).and_then(|c| c.tiles.get(c.sel)).cloned() else {
            return;
        };
        self.sounds.play("select");
        match tile.action {
            Action::Launch(i) => {
                let s = self.games[i].system;
                if self.cfg.launcher(s) == config::Launcher::None {
                    self.no_emulator_dialog(s, i);
                    return;
                }
                self.sounds.play("launch");
                self.launching = Some((Instant::now(), Some(i), Some(s)));
            }
            Action::GoToChannel(c) => {
                self.set_chan(c.min(self.channels.len() - 1));
                self.sounds.play("channel");
            }
            Action::AddFolder(s) => self.add_folder(s, frame),
            Action::OpenSettings => self.menu = Some(menu::Menu::new(menu::Kind::Settings)),
            Action::OpenSetup => self.menu = Some(menu::Menu::new(menu::Kind::Setup)),
            Action::Rescan => {
                self.cfg = Config::load();
                self.rescan();
                self.toast(format!("{} games found", self.games.len()));
            }
            Action::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    /// Start (or prepare) the built-in core for a console, then play `game` when it's ready.
    pub fn install_core(&mut self, s: System, game: Option<usize>, visible: bool) {
        if self.installing.as_ref().map(|i| i.system == s).unwrap_or(false) {
            if let Some(i) = self.installing.as_mut() {
                i.game = game.or(i.game);
                i.visible |= visible;
            }
            return;
        }
        let _ = self.rt_tx.send(runtime::Job::Install(s));
        self.installing = Some(Installing {
            system: s,
            game,
            stage: "Getting ready".into(),
            frac: 0.0,
            shown: 0.0,
            error: None,
            visible,
            started: Instant::now(),
        });
    }

    fn poll_runtime(&mut self) {
        let msgs: Vec<runtime::Msg> = self.rt_rx.try_iter().collect();
        for m in msgs {
            match m {
                runtime::Msg::Progress { system, stage, frac } => {
                    if let Some(i) = self.installing.as_mut().filter(|i| i.system == system) {
                        i.stage = stage;
                        i.frac = frac;
                    }
                }
                runtime::Msg::Done { system, result } => match result {
                    Ok(()) => {
                        let next = self.installing.take().filter(|i| i.system == system).and_then(|i| i.game.filter(|_| i.visible));
                        self.rebuild();
                        match next {
                            // Straight into the game, no extra button press.
                            Some(g) => self.launching = Some((Instant::now(), Some(g), Some(system))),
                            None => self.toast(format!("{} core ready", system.short())),
                        }
                    }
                    Err(e) => {
                        if let Some(i) = self.installing.as_mut() {
                            i.error = Some(e.clone());
                            i.visible = true;
                        }
                    }
                },
            }
        }
    }

    fn spawn_game(&mut self, ctx: &egui::Context, game: Option<usize>, system: System) {
        let (mut cmd, name, workdir) = match self.cfg.launcher(system) {
            config::Launcher::Emulator(emu) => {
                let mut cmd = Command::new(&emu);
                let user_args = &self.cfg.sys(system).args;
                let args = if user_args.is_empty() { system.default_args(&emu) } else { user_args.clone() };
                let name = match game.and_then(|i| self.games.get(i)) {
                    Some(g) => {
                        cmd.args(&args);
                        cmd.arg(&g.path);
                        g.name.clone()
                    }
                    None => emu_name(Some(&emu)),
                };
                (cmd, name, emu.parent().map(|p| p.to_path_buf()))
            }
            config::Launcher::Core => {
                if !runtime::installed(system) {
                    self.install_core(system, game, true);
                    return;
                }
                let Some(g) = game.and_then(|i| self.games.get(i)) else { return };
                let exe = runtime::player_exe();
                let mut cmd = Command::new(&exe);
                cmd.arg("-L").arg(runtime::core_path(system).unwrap()).arg("--config").arg(runtime::write_config(&self.cfg.ra_user, &self.cfg.ra_token)).arg("-f").arg(&g.path);
                (cmd, g.name.clone(), exe.parent().map(|p| p.to_path_buf()))
            }
            config::Launcher::None => return,
        };
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }
        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                if let Some(g) = game.and_then(|i| self.games.get(i)) {
                    let p = g.path.to_string_lossy().to_string();
                    self.cfg.push_recent(&p);
                }
                *self.shared.child.lock().unwrap() = Some(child);
                self.shared.running.store(true, Ordering::Relaxed);
                self.playing = Some(name);
                if self.cfg.minimize_while_playing {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                // Make sure the emulator ends up in front once its window exists.
                let shared = self.shared.clone();
                std::thread::spawn(move || {
                    for _ in 0..60 {
                        std::thread::sleep(Duration::from_millis(250));
                        if !shared.running.load(Ordering::Relaxed) {
                            return;
                        }
                        if winapi::focus_pid(pid) {
                            std::thread::sleep(Duration::from_millis(1500));
                            winapi::focus_pid(pid);
                            return;
                        }
                    }
                });
            }
            Err(e) => self.toast(format!("Couldn't start {name}: {e}")),
        }
    }

    fn handle(&mut self, p: Pad, ctx: &egui::Context, frame: &eframe::Frame) {
        if self.launching.is_some() || self.playing.is_some() {
            return;
        }
        if self.cropper.is_some() {
            self.cropper_input(p);
            return;
        }
        if self.pic_picker.is_some() {
            self.pic_picker_input(p, frame, ctx);
            return;
        }
        if self.dialog.is_some() {
            self.dialog_input(p, frame);
            return;
        }
        if let Some(i) = self.installing.as_mut().filter(|i| i.visible) {
            if p == Pad::B {
                // Keep downloading quietly, just don't jump into the game afterwards.
                i.visible = false;
                i.game = None;
                if i.error.is_some() {
                    self.installing = None;
                }
                self.sounds.play("back");
            }
            return;
        }
        if self.picker.is_some() {
            self.picker_input(p);
            return;
        }
        if self.ach.is_some() {
            self.ach_input(p);
            return;
        }
        if self.menu.is_some() {
            self.menu_input(p, frame, ctx);
            return;
        }
        let n = self.channels.len();
        match p {
            // The following channels are listed above the current one, so Up
            // brings the one above down into place (and Down goes back).
            Pad::Up if n > 1 => {
                self.step_chan(1);
                self.sounds.play("channel");
            }
            Pad::Down if n > 1 => {
                self.step_chan(-1);
                self.sounds.play("channel");
            }
            Pad::Left => {
                let c = &mut self.channels[self.chan];
                if c.sel > 0 {
                    c.sel -= 1;
                    self.sel_changed = Instant::now();
                    self.sounds.play("move");
                }
            }
            Pad::Right => {
                let c = &mut self.channels[self.chan];
                if c.sel + 1 < c.tiles.len() {
                    c.sel += 1;
                    self.sel_changed = Instant::now();
                    self.sounds.play("move");
                }
            }
            Pad::A => self.activate(ctx, frame),
            Pad::Start => {
                self.menu = Some(menu::Menu::new(menu::Kind::Settings));
                self.sounds.play("select");
            }
            Pad::Y => {
                if let Some(gi) = self.focused_game() {
                    self.open_picker(gi);
                }
            }
            Pad::X => {
                if let Some(gi) = self.focused_game() {
                    self.open_achievements(gi);
                }
            }
            Pad::B => {
                self.sounds.play("back");
                let c = &mut self.channels[self.chan];
                if c.sel > 0 {
                    c.sel = 0;
                } else {
                    self.set_chan(0);
                }
            }
            Pad::Guide => {
                self.set_chan(0);
                self.channels[0].sel = 0;
                self.sounds.play("channel");
            }
            _ => {}
        }
    }

    pub fn focused_game(&self) -> Option<usize> {
        self.channels.get(self.chan).and_then(|c| c.tiles.get(c.sel)).and_then(|t| t.game)
    }

    /// (texture, aspect ratio, average colour) for a game's cover.
    pub fn cover(&mut self, ctx: &egui::Context, gi: usize) -> Option<(egui::TextureId, f32, egui::Color32)> {
        if !self.covers.contains_key(&gi) {
            let tex = self.games.get(gi).and_then(|g| g.cover.as_ref()).and_then(|bytes| {
                let img = image::load_from_memory(bytes).ok()?.thumbnail(640, 640).to_rgba8();
                // Average colour (from a tiny copy) tints the tile behind the art.
                let tiny = image::imageops::thumbnail(&img, 8, 8);
                let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
                for px in tiny.pixels() {
                    r += px[0] as u32;
                    g += px[1] as u32;
                    b += px[2] as u32;
                }
                let n = (tiny.width() * tiny.height()).max(1);
                let avg = egui::Color32::from_rgb((r / n) as u8, (g / n) as u8, (b / n) as u8);
                let size = [img.width() as usize, img.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                Some((ctx.load_texture(format!("cover{gi}"), ci, egui::TextureOptions::LINEAR), avg))
            });
            self.covers.insert(gi, tex);
        }
        self.covers.get(&gi).and_then(|t| {
            t.as_ref().map(|(t, c)| {
                let [w, h] = t.size();
                (t.id(), w as f32 / h.max(1) as f32, *c)
            })
        })
    }
}

fn emu_name(p: Option<&std::path::Path>) -> String {
    p.and_then(|p| p.file_stem())
        .map(|s| {
            let s = s.to_string_lossy().to_string();
            let s = s.replace(['_', '-'], " ");
            let mut c = s.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => s,
            }
        })
        .unwrap_or_else(|| "emulator".into())
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Runs even while minimized: notice when the emulator exits and come back.
        if self.shared.running.load(Ordering::Relaxed) {
            let mut lock = self.shared.child.lock().unwrap();
            let done = match lock.as_mut() {
                Some(c) => !matches!(c.try_wait(), Ok(None)),
                None => true,
            };
            if done {
                *lock = None;
                drop(lock);
                self.shared.running.store(false, Ordering::Relaxed);
                self.playing = None;
                self.rebuild();
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                winapi::focus_pid(std::process::id());
                self.returned = Some(Instant::now());
                self.sounds.play("back");
            } else {
                ctx.request_repaint_after(Duration::from_millis(500));
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.poll_covers(&ctx);
        self.poll_ra(&ctx);
        self.poll_runtime();
        self.ra_prefetch();

        // Input: gamepad thread + keyboard. While typing in a text box only Enter/Esc count.
        let editing = self.menu.as_ref().map(|m| m.edit.is_some()).unwrap_or(false);
        let mut pads: Vec<Pad> = self.rx.try_iter().collect();
        ctx.input(|i| {
            use egui::Key::*;
            let map: &[(egui::Key, Pad)] = if editing {
                &[(Enter, Pad::A), (Escape, Pad::B)]
            } else {
                &[
                    (ArrowUp, Pad::Up),
                    (ArrowDown, Pad::Down),
                    (ArrowLeft, Pad::Left),
                    (ArrowRight, Pad::Right),
                    (Enter, Pad::A),
                    (Space, Pad::A),
                    (Escape, Pad::B),
                    (Backspace, Pad::B),
                    (Tab, Pad::Start),
                    (C, Pad::Y),
                    (X, Pad::X),
                    (Q, Pad::Lb),
                    (E, Pad::Rb),
                    (Minus, Pad::Lb),
                    (Plus, Pad::Rb),
                    (Equals, Pad::Rb),
                    (Home, Pad::Guide),
                ]
            };
            for (k, p) in map {
                if i.key_pressed(*k) {
                    pads.push(*p);
                }
            }
        });
        if !editing && ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }
        for p in pads {
            self.handle(p, &ctx, frame);
        }

        // Hide the mouse pointer unless it's being moved.
        if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
            if pos != self.last_pointer.0 {
                self.last_pointer = (pos, now);
            }
        }
        if self.last_pointer.1.elapsed() > Duration::from_secs(2) {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        // Launch fade, then spawn.
        if let Some((t, g, s)) = self.launching {
            if t.elapsed().as_secs_f32() > 0.6 {
                self.launching = None;
                if let Some(s) = s {
                    self.spawn_game(&ctx, g, s);
                }
            }
        }

        // Animations.
        // Critically damped springs, sub-stepped so motion is identical at 60, 144 or 240 Hz.
        let spring = |x: &mut f32, v: &mut f32, target: f32, w: f32| {
            let steps = (dt / (1.0 / 480.0)).ceil().max(1.0) as usize;
            let h = dt / steps as f32;
            for _ in 0..steps {
                let acc = w * w * (target - *x) - 2.0 * w * *v;
                *v += acc * h;
                *x += *v * h;
            }
            if (target - *x).abs() < 0.0005 && v.abs() < 0.001 {
                *x = target;
                *v = 0.0;
            }
        };
        spring(&mut self.chan_anim, &mut self.chan_vel, self.chan_pos, 17.0);
        for c in &mut self.channels {
            spring(&mut c.anim, &mut c.vel, c.sel as f32, 19.0);
        }

        // ---- PS4-style page zoom.
        // Opening a page: the dashboard pushes towards you and fades while the page grows in
        // from slightly smaller. Moving deeper/back between pages does a smaller zoom the same way.
        let top = self.top_page();
        let page_target = if top.is_some() { 1.0 } else { 0.0 };
        spring(&mut self.page_anim, &mut self.page_vel, page_target, 13.0);
        if top != self.last_top {
            if let (Some(old), Some(new)) = (&self.last_top, &top) {
                // Deeper pages grow in, going back shrinks in from larger.
                self.sub_from = if page_depth(new) >= page_depth(old) { 0.94 } else { 1.06 };
                self.sub_anim = 0.0;
                self.sub_vel = 0.0;
            }
            if top.is_some() {
                self.last_page = top.clone();
            }
            self.last_top = top.clone();
        }
        spring(&mut self.sub_anim, &mut self.sub_vel, 1.0, 16.0);

        let screen = ctx.content_rect();
        let centre = screen.center().to_vec2();
        let zoom = |s: f32| {
            egui::emath::TSTransform::from_translation(centre) * egui::emath::TSTransform::from_scaling(s) * egui::emath::TSTransform::from_translation(-centre)
        };
        let pa = self.page_anim.clamp(0.0, 1.2);
        let dash_layer = egui::LayerId::new(egui::Order::Middle, nxe_id("nxe_dash"));
        let page_layer = egui::LayerId::new(egui::Order::Foreground, nxe_id("nxe_page"));
        let top_layer = egui::LayerId::new(egui::Order::Tooltip, nxe_id("nxe_top"));
        ctx.set_transform_layer(dash_layer, zoom(1.0 + 0.09 * pa));
        let sub = self.sub_from + (1.0 - self.sub_from) * self.sub_anim.clamp(0.0, 1.0);
        ctx.set_transform_layer(page_layer, zoom((0.9 + 0.1 * pa) * sub));

        scene::draw_background(self, ui, &ctx);
        let dash_alpha = (1.0 - pa * 1.15).clamp(0.0, 1.0);
        if dash_alpha > 0.0 {
            ui.scope_builder(egui::UiBuilder::new().layer_id(dash_layer).max_rect(screen), |ui| {
                ui.multiply_opacity(dash_alpha);
                scene::draw(self, ui, &ctx);
            });
        }
        let page_alpha = pa.clamp(0.0, 1.0).powf(0.8) * (0.35 + 0.65 * self.sub_anim.clamp(0.0, 1.0));
        let shown = top.clone().or_else(|| if pa > 0.01 { self.last_page.clone() } else { None });
        if shown.is_some() {
            ui.scope_builder(egui::UiBuilder::new().layer_id(page_layer).max_rect(screen), |ui| {
                ui.multiply_opacity(page_alpha);
                // Only the top-most page is drawn; the ones under it come back when it closes.
                match top.as_ref().map(|k| k.split(':').next().unwrap_or("")) {
                    Some("crop") => profile::draw_cropper(self, ui, &ctx),
                    Some("pics") => profile::draw_pic_picker(self, ui, &ctx),
                    Some("ach") => menu::draw_achievements(self, ui, &ctx),
                    Some("cover") => menu::draw_picker(self, ui, &ctx),
                    _ => menu::draw(self, ui, &ctx),
                }
            });
        }
        ui.scope_builder(egui::UiBuilder::new().layer_id(top_layer).max_rect(screen), |ui| {
            menu::draw_dialog(self, ui, &ctx);
        });

        // Finish closing a fading menu; quick setup hands over to the dashboard intro.
        if let Some(c) = self.menu.as_ref().and_then(|m| m.closing) {
            if c.elapsed().as_secs_f32() >= menu::CLOSE_TIME {
                self.menu = None;
            }
        }

        // Version mark, bottom-right on every screen (off by default).
        if self.cfg.show_version {
            let screen = ctx.content_rect();
            let u = (screen.height() / 720.0).min(screen.width() / 1280.0);
            let p = ui.painter_at(screen);
            let pos = egui::pos2(screen.right() - 22.0 * u, screen.bottom() - 16.0 * u);
            let txt = concat!("NXE Launcher v", env!("CARGO_PKG_VERSION"));
            p.text(pos + egui::vec2(1.0, 1.0), egui::Align2::RIGHT_BOTTOM, txt, egui::FontId::proportional(13.0 * u), egui::Color32::from_black_alpha(70));
            p.text(pos, egui::Align2::RIGHT_BOTTOM, txt, egui::FontId::proportional(13.0 * u), egui::Color32::from_white_alpha(150));
        }

        // Repaint at the display's refresh rate (vsync), or at the chosen cap.
        self.last_paint = Instant::now();
        if self.playing.is_some() {
            ctx.request_repaint_after(Duration::from_millis(500));
        } else if self.cfg.frame_rate > 0 {
            ctx.request_repaint_after(Duration::from_secs_f32(1.0 / self.cfg.frame_rate as f32));
        } else {
            ctx.request_repaint();
        }
    }
}

// accessors for scene.rs
impl App {
    pub fn parts(&self) -> (&Config, &[Channel], usize, f32) {
        (&self.cfg, &self.channels, self.chan, self.chan_anim)
    }
    /// Identifies the page on top ("menu:..." / "cover" / "ach" / "pics" / "crop"), None = dashboard.
    fn top_page(&self) -> Option<String> {
        if self.cropper.is_some() {
            return Some("crop".into());
        }
        if self.pic_picker.is_some() {
            return Some("pics".into());
        }
        if self.ach.is_some() {
            return Some("ach".into());
        }
        if self.picker.is_some() {
            return Some("cover".into());
        }
        let m = self.menu.as_ref()?;
        if m.closing.is_some() {
            return None;
        }
        Some(menu::page_key(m))
    }

    pub fn background(&self) -> Option<egui::TextureId> {
        self.background.as_ref().map(|t| t.id())
    }
    pub fn time(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }
    pub fn launching(&self) -> Option<(f32, String)> {
        self.launching.map(|(t, g, s)| {
            let name = g
                .and_then(|i| self.games.get(i))
                .map(|g| g.name.clone())
                .unwrap_or_else(|| s.and_then(|s| self.cfg.emulator(s)).map(|e| emu_name(Some(&e))).unwrap_or_default());
            (t.elapsed().as_secs_f32(), name)
        })
    }
    pub fn sel_age(&self) -> f32 {
        self.sel_changed.elapsed().as_secs_f32()
    }
    pub fn returned(&self) -> Option<f32> {
        self.returned.map(|t| t.elapsed().as_secs_f32()).filter(|t| *t < 0.9)
    }
    pub fn playing(&self) -> Option<&str> {
        self.playing.as_deref()
    }
    pub fn menu_open(&self) -> bool {
        self.menu.as_ref().map(|m| m.closing.is_none()).unwrap_or(false)
            || self.picker.is_some()
            || self.ach.is_some()
            || self.dialog.is_some()
            || self.pic_picker.is_some()
            || self.cropper.is_some()
    }
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
    pub fn channels_ref(&self) -> &[Channel] {
        &self.channels
    }
    pub fn chan_index(&self) -> usize {
        self.chan
    }
    pub fn chan_anim(&self) -> f32 {
        self.chan_anim
    }
    /// Channels whose tile row is on screen: (index, signed offset -1..1).
    pub fn visible_channels(&self, anim: f32) -> Vec<(usize, f32)> {
        let n = self.channels.len() as f32;
        (0..self.channels.len())
            .filter_map(|c| {
                let mut o = c as f32 - anim;
                o -= n * (o / n).round();
                (o.abs() < 1.0).then_some((c, o))
            })
            .collect()
    }
    pub fn focused_progress(&self) -> Option<(u32, u32)> {
        let gi = self.focused_game()?;
        let id = *self.ra_match.get(&gi)?;
        self.ra_progress.get(&id).copied().filter(|(_, t)| *t > 0)
    }
    pub fn ra_points(&self) -> Option<u64> {
        self.ra_points_val.filter(|_| self.ra_signed_in())
    }
    pub fn ra_avatar(&self) -> Option<egui::TextureId> {
        self.ra_avatar_tex.as_ref().map(|t| t.id())
    }
    pub fn toast_text(&self) -> Option<(&str, f32)> {
        self.toast.as_ref().map(|(s, t)| (s.as_str(), t.elapsed().as_secs_f32())).filter(|(_, t)| *t < 3.0)
    }
}

fn load_texture(ctx: &egui::Context, name: &str, path: &std::path::Path) -> Option<egui::TextureHandle> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    Some(ctx.load_texture(name, ci, egui::TextureOptions::LINEAR))
}

fn install_font(ctx: &egui::Context) {
    // theme\font.ttf, else Segoe UI (closest system font to the console's look on Windows).
    let mut cands = vec![theme_dir().join("font.ttf")];
    if let Ok(win) = std::env::var("WINDIR") {
        cands.push(PathBuf::from(&win).join("Fonts").join("segoeui.ttf"));
    }
    let Some(bytes) = cands.iter().find_map(|p| std::fs::read(p).ok()) else { return };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("theme".into(), Arc::new(egui::FontData::from_owned(bytes)));
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.insert(0, "theme".into());
    }
    ctx.set_fonts(fonts);
}

pub fn open_path(p: &std::path::Path) {
    #[cfg(windows)]
    let _ = Command::new("notepad.exe").arg(p).spawn();
    #[cfg(not(windows))]
    let _ = Command::new("xdg-open").arg(p).spawn();
}

fn load_tex(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
    Some(ctx.load_texture(name, ci, egui::TextureOptions::LINEAR))
}

#[allow(deprecated)]
fn nxe_id(s: &str) -> egui::Id {
    egui::Id::new(s)
}

fn page_depth(k: &str) -> u8 {
    match k.split(':').next().unwrap_or("") {
        "menu" => k.matches(':').count() as u8,
        _ => 9,
    }
}

fn main() -> eframe::Result {
    let cfg = Config::load();
    let _ = std::fs::create_dir_all(theme_dir().join("covers"));
    let _ = std::fs::create_dir_all(theme_dir().join("sounds"));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NXE Launcher")
            .with_inner_size([1600.0, 900.0])
            .with_fullscreen(cfg.fullscreen),
        ..Default::default()
    };
    eframe::run_native("NXE Launcher", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
