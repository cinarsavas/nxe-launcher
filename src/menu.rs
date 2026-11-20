//! Settings, quick setup, cover picker and achievements, drawn like the
//! console's own system screens: big title top-left, a slate panel with a
//! list on the left (green bar on the selected row) and "Current Setting" /
//! description on the right, button hints bottom-left.

use crate::config::config_path;
use crate::covers;
use crate::input::Pad;
use crate::scene::{self, fit, hint_bar, text_shadow, BTN_A, BTN_B};
use crate::config::Launcher;
use crate::systems::{self, System};
use crate::{theme_dir, App, Icon};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Settings,
    Setup,
}

pub struct Menu {
    pub kind: Kind,
    pub cat: usize,
    pub row: usize,
    /// false = choosing a category (top level), true = inside one.
    pub in_list: bool,
    pub edit: Option<Edit>,
    pub opened: Instant,
    pub scroll: usize,
    /// Animated position of the green selection bar (in visible-row units).
    pub bar: f32,
    pub bar_vel: f32,
    pub last_frame: Instant,
    /// Set while the menu fades out.
    pub closing: Option<Instant>,
    /// Inside Settings > Consoles > this console.
    pub console: Option<System>,
}

pub const CLOSE_TIME: f32 = 0.3;

pub struct Edit {
    pub target: RowId,
    pub text: String,
    pub focus_set: bool,
    pub widget: Option<egui::Id>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RowId {
    Gamertag,
    Theme,
    Fullscreen,
    Sounds,
    Minimize,
    RunSetup,
    Rescan,
    OpenFile,
    Exit,
    Emulator(System),
    Args(System),
    Import(System),
    Folder(System, usize),
    AddFolder(System),
    AutoCovers,
    FetchCovers,
    ResetCovers,
    RaUser,
    RaPassword,
    RaSignIn,
    RaRefresh,
    RaSignOut,
    OpenConsole(System),
    EmuOpt(System, usize),
    OpenEmu(System),
    EmulatorExe(System),
    EmulatorFolder(System),
    EmulatorClear(System),
    Gamerpic,
    CustomPic,
    FrameRate,
    ShowVersion,
    Backend(System),
    InstallCore(System),
    Continue,
    Back,
    Finish,
    None,
}

pub enum Value {
    Action,
    Text(String),
    Toggle(bool),
    Status(bool, String),
    /// Shown in the right-hand column, not as a row.
    Note,
}

pub struct Row {
    pub id: RowId,
    pub label: String,
    pub value: Value,
    pub desc: String,
}

impl Row {
    fn new(id: RowId, label: impl Into<String>, value: Value, desc: impl Into<String>) -> Self {
        Self { id, label: label.into(), value, desc: desc.into() }
    }
    fn note(text: impl Into<String>) -> Self {
        Self::new(RowId::None, text, Value::Note, "")
    }
    pub fn selectable(&self) -> bool {
        !matches!(self.value, Value::Note)
    }
}

/// Top-level settings categories.
fn settings_cats() -> Vec<(String, Icon, String)> {
    vec![
        ("General".into(), Icon::Gear, "Rescan your games, run Quick Setup again, or close the dashboard.".into()),
        ("Consoles".into(), Icon::Disc, "Set up each console: which emulator to use, where your games are, built-in cores and graphics settings.".into()),
        ("Profile".into(), Icon::Info, "Your name and gamerpic.".into()),
        ("Display".into(), Icon::Expand, "Theme colour, fullscreen, animation frame rate and sounds.".into()),
        ("Cover Art".into(), Icon::Image, "Download box art automatically and fix wrong covers.".into()),
        ("Achievements".into(), Icon::Trophy, "Sign in to RetroAchievements to see your points and progress.".into()),
        ("About".into(), Icon::Info, "Version and controls.".into()),
    ]
}
const CAT_GENERAL: usize = 0;
const CAT_CONSOLES: usize = 1;
const CAT_PROFILE: usize = 2;
const CAT_DISPLAY: usize = 3;
const CAT_COVERS: usize = 4;
const CAT_ACH: usize = 5;
const SETUP_STEPS: [&str; 4] = ["Welcome", "Gamerpic", "Emulators", "Cover Art"];

fn emu_label(s: System) -> &'static str {
    s.emu_label()
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

impl Menu {
    pub fn new(kind: Kind) -> Self {
        Self { kind, cat: 0, row: 0, in_list: kind == Kind::Setup, edit: None, opened: Instant::now(), scroll: 0, bar: 0.0, bar_vel: 0.0, last_frame: Instant::now(), closing: None, console: None }
    }
    fn ncats(&self) -> usize {
        match self.kind {
            Kind::Settings => settings_cats().len(),
            Kind::Setup => SETUP_STEPS.len(),
        }
    }
}

// ---------------------------------------------------------------- rows

pub fn rows(app: &App, m: &Menu) -> Vec<Row> {
    let cfg = &app.cfg;
    let mut v = Vec::new();
    let onoff = Value::Toggle;
    let count = |s: System| app.games.iter().filter(|g| g.system == s).count();
    match m.kind {
        Kind::Settings if m.console.is_some() => console_rows(app, m.console.unwrap(), &mut v),
        Kind::Settings => match m.cat {
            CAT_GENERAL => {
                v.push(Row::new(RowId::Rescan, "Rescan Library", Value::Action, format!("Look through your game folders again for new games. {} games found right now.", app.games.len())));
                v.push(Row::new(RowId::RunSetup, "Quick Setup", Value::Action, "Go through the first-launch setup again: your name, theme, emulators and cover art."));
                v.push(Row::new(RowId::OpenFile, "Open Settings File", Value::Action, format!("For advanced changes. Everything here is also saved in {}", config_path().to_string_lossy())));
                v.push(Row::new(RowId::Exit, "Exit Dashboard", Value::Action, "Close NXE Launcher."));
            }
            CAT_CONSOLES => {
                for s in systems::ALL {
                    let (ok, how) = match cfg.launcher(s) {
                        Launcher::Emulator(_) => (true, format!("Ready: plays with {}.", emu_label(s))),
                        Launcher::Core => (true, format!("Ready: plays with the built-in {}.", s.core().map(|c| c.name).unwrap_or(""))),
                        Launcher::None => (false, format!("Not set up yet: no {} found.", emu_label(s))),
                    };
                    let n = count(s);
                    v.push(Row::new(
                        RowId::OpenConsole(s),
                        s.channel(),
                        Value::Status(ok, String::new()),
                        format!("{how} {n} game(s) found. Press A to set it up."),
                    ));
                }
            }
            CAT_PROFILE => {
                v.push(Row::new(RowId::Gamertag, "Gamertag", Value::Text(cfg.gamertag.clone()), "Your name, shown in the top-right corner. Press A, type, then press Enter."));
                v.push(Row::new(
                    RowId::Gamerpic,
                    "Gamerpic",
                    Value::Text(crate::profile::current_name(app)),
                    "Pick one of the built-in gamerpics, or one from your theme\\gamerpics folder.",
                ));
                v.push(Row::new(RowId::CustomPic, "Use My Own Picture", Value::Action, "Choose any image from your PC, then move and zoom it until it looks right."));
            }
            CAT_DISPLAY => {
                v.push(Row::new(RowId::Theme, "Theme", Value::Text(cap(&cfg.theme)), "Colour of the dashboard. Press A to switch."));
                v.push(Row::new(
                    RowId::FrameRate,
                    "Animation Frame Rate",
                    Value::Text(if cfg.frame_rate == 0 { "Match Display".into() } else { format!("{} FPS", cfg.frame_rate) }),
                    "Match Display animates at your monitor's refresh rate (60, 144, 180, 240 Hz…). Lower it to save power on laptops.",
                ));
                v.push(Row::new(RowId::Fullscreen, "Fullscreen", onoff(cfg.fullscreen), "Run the dashboard fullscreen. F11 also switches it."));
                v.push(Row::new(RowId::Sounds, "Menu Sounds", onoff(cfg.sounds), "Plays theme\\sounds\\*.wav when you move around."));
                v.push(Row::new(RowId::ShowVersion, "Show Version Number", onoff(cfg.show_version), "Shows the launcher version in the bottom-right corner."));
                v.push(Row::new(
                    RowId::Minimize,
                    "Minimize While Playing",
                    onoff(cfg.minimize_while_playing),
                    "Hide the dashboard while a game runs. Off keeps it ready behind the game for a faster return.",
                ));
            }
            CAT_COVERS => {
                v.push(Row::new(
                    RowId::AutoCovers,
                    "Download Covers",
                    onoff(cfg.auto_covers),
                    "Find box art online for games that don't have a cover.",
                ));
                v.push(Row::new(RowId::FetchCovers, "Find Missing Covers", Value::Action, "Search again for games that still have no cover."));
                v.push(Row::new(RowId::ResetCovers, "Clear Downloaded Covers", Value::Action, "Delete automatic downloads (not ones you picked) and search again."));
                let (done, queued) = app.cover_progress;
                v.push(Row::note(if queued > 0 { format!("Downloading: {done} checked, {queued} waiting.") } else { format!("{done} game(s) checked this session.") }));
                v.push(Row::note("Wrong cover? Highlight the game and press Y."));
            }
            CAT_ACH => {
                if app.ra_signed_in() {
                    v.push(Row::new(
                        RowId::None,
                        format!("Signed in as {}", cfg.ra_user),
                        Value::Action,
                        app.ra_status.clone().unwrap_or_default(),
                    ));
                    v.push(Row::new(RowId::RaRefresh, "Refresh", Value::Action, "Reload your points and progress."));
                    v.push(Row::new(RowId::RaSignOut, "Sign Out", Value::Action, "Forget the saved sign-in on this PC."));
                } else {
                    v.push(Row::note("Sign in with your retroachievements.org account, the same username and password you use on the website."));
                    v.push(Row::new(
                        RowId::RaUser,
                        "Username",
                        Value::Text(cfg.ra_user.clone()),
                        "Step 1: press A, type your RetroAchievements username, press Enter.",
                    ));
                    v.push(Row::new(
                        RowId::RaPassword,
                        "Password",
                        Value::Text("•".repeat(app.ra_password.chars().count())),
                        "Step 2: press A, type your password, press Enter. It's only sent to RetroAchievements to sign in and is never saved.",
                    ));
                    v.push(Row::new(RowId::RaSignIn, "Sign In", Value::Action, "Step 3: press A to sign in."));
                    if let Some(st) = &app.ra_status {
                        v.push(Row::note(st.clone()));
                    }
                }
                match crate::ra::pcsx2_status(cfg.emulator(System::Ps2).as_deref()) {
                    Some((true, u)) => v.push(Row::note(format!("PCSX2 is signed in as {u}, so PS2 achievements unlock while you play."))),
                    Some((false, _)) => v.push(Row::note("PCSX2 isn't signed in yet. Open PCSX2 > Settings > Achievements and sign in once.")),
                    None => {}
                }
                v.push(Row::note("Achievements exist for PS1, PS2, PSP, GameCube, N64 and DS games."));
            }
            _ => {
                v.push(Row::new(
                    RowId::None,
                    concat!("NXE Launcher v", env!("CARGO_PKG_VERSION")),
                    Value::Action,
                    "A console-style dashboard for your emulators.",
                ));
                v.push(Row::note("D-pad / stick: move · A: select · B: back"));
                v.push(Row::note("X: achievements · Y: change cover · Start: settings"));
                v.push(Row::note("Up / Down: switch channels"));
                v.push(Row::note("In a game: Guide, or hold Back + Start, returns to the dashboard."));
                v.push(Row::note("Keyboard: arrows, Enter, Esc, Tab, X, C, F11."));
            }
        },
        Kind::Setup => match m.cat {
            1 => {
                v.push(Row::note("Your gamerpic is the picture next to your name on the dashboard."));
                v.push(Row::new(RowId::Gamerpic, "Gamerpic", Value::Text(crate::profile::current_name(app)), "Press A to see all the gamerpics and pick one."));
                v.push(Row::new(RowId::CustomPic, "Use My Own Picture", Value::Action, "Choose any image from your PC, then move and zoom it to fit."));
                v.push(Row::new(RowId::Continue, "Continue", Value::Action, ""));
                v.push(Row::new(RowId::Back, "Back", Value::Action, ""));
            }
            0 => {
                v.push(Row::note("Let's get your consoles ready. You can change everything later in Settings."));
                v.push(Row::new(RowId::Gamertag, "Your Name", Value::Text(cfg.gamertag.clone()), "Shown in the corner of the dashboard. Press A to type."));
                v.push(Row::new(RowId::Theme, "Theme", Value::Text(cap(&cfg.theme)), "Press A to switch colour."));
                v.push(Row::new(RowId::Continue, "Continue", Value::Action, ""));
            }
            2 => {
                v.push(Row::note("Emulators in folders next to the launcher are found automatically. Skip consoles you don't use."));
                for s in systems::ALL {
                    let emu = cfg.emulator(s);
                    let txt = match &emu {
                        Some(p) => p.to_string_lossy().to_string(),
                        None => "Not found. Press A to choose it.".into(),
                    };
                    v.push(Row::new(
                        RowId::Emulator(s),
                        format!("{} ({})", s.short(), emu_label(s)),
                        Value::Status(emu.is_some(), txt),
                        if s.core().is_some() {
                            "Press A to choose the emulator. No emulator? You can download a built-in core later in Settings."
                        } else {
                            "Press A to choose the emulator."
                        },
                    ));
                }
                v.push(Row::new(RowId::Continue, "Continue", Value::Action, ""));
                v.push(Row::new(RowId::Back, "Back", Value::Action, ""));
            }
            _ => {
                v.push(Row::note("Box art can be downloaded for your games. If one is wrong, press Y on that game to pick another."));
                v.push(Row::note("Game folders are in Settings, under each console. Games your emulators already know about show up by themselves."));
                v.push(Row::new(RowId::AutoCovers, "Download Covers", onoff(cfg.auto_covers), "Uses the internet. Covers are saved in theme\\cache."));
                v.push(Row::new(RowId::Finish, "Finish", Value::Action, ""));
                v.push(Row::new(RowId::Back, "Back", Value::Action, ""));
            }
        },
    }
    v
}

/// One console's settings page (Settings > Consoles > [console]).
fn console_rows(app: &App, s: System, v: &mut Vec<Row>) {
    let cfg = &app.cfg;
    let emu = cfg.emulator(s);
    let sc = cfg.sys(s);
    let count = app.games.iter().filter(|g| g.system == s).count();
    v.push(Row::new(
        RowId::Emulator(s),
        "Emulator",
        Value::Status(
            emu.is_some(),
            emu.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "Not set".into()),
        ),
        format!(
            "Tell the launcher where {} is. Press A, then pick either the program file itself ({}) or the folder it's in; both work. Tip: emulators in folders next to the launcher are found by themselves.",
            emu_label(s),
            s.emulator_hint()
        ),
    ));
    if let Some(core) = s.core() {
        let mode = match sc.backend.as_str() {
            "core" => "Built-in Core".to_string(),
            "emulator" => format!("Emulator ({})", emu_label(s)),
            _ => match &emu {
                Some(_) => format!("Auto: {}", emu_label(s)),
                None if crate::runtime::installed(s) => "Auto: Built-in Core".to_string(),
                None => "Auto".to_string(),
            },
        };
        v.push(Row::new(
            RowId::Backend(s),
            "Play With",
            Value::Text(mode),
            "Auto: use your emulator if you have one, otherwise the built-in core (only if you downloaded it). Press A to force one or the other.",
        ));
        let ok = crate::runtime::installed(s);
        v.push(Row::new(
            RowId::InstallCore(s),
            "Built-in Core",
            Value::Status(ok, if ok { format!("{} (downloaded)", core.name) } else { format!("{} (not downloaded)", core.name) }),
            if ok {
                format!("Lets this console run without installing an emulator. {}", core.note)
            } else {
                format!("Optional. Press A to download it so this console runs without installing an emulator. {}", core.note)
            },
        ));
    }
    // Basic emulator settings written to the emulator's own config.
    if let Some(e) = &emu {
        let opts = crate::emusettings::options(s, e);
        for (i, o) in opts.iter().enumerate() {
            v.push(Row::new(RowId::EmuOpt(s, i), o.label, Value::Text(crate::emusettings::current(o)), format!("{} Saved in {}'s own settings.", o.desc, emu_label(s))));
        }
        if opts.is_empty() && crate::emusettings::config_file(s, e).is_none() && matches!(s, System::Ps2 | System::Gc | System::Ps1 | System::Psp | System::Xbox360 | System::Xbox) {
            v.push(Row::note(format!("Graphics options show up here after {} has been opened once.", emu_label(s))));
        }
        v.push(Row::new(
            RowId::OpenEmu(s),
            "Controls & More Settings",
            Value::Action,
            format!("Opens {} so you can set up your controller and anything else. Close it to come back here.", emu_label(s)),
        ));
    }
    v.push(Row::new(
        RowId::AddFolder(s),
        "Add Game Folder",
        Value::Action,
        format!("Choose a folder that has your {} games ({}). Sub-folders are searched too.", s.short(), s.folder_hint()),
    ));
    for (i, d) in sc.game_dirs.iter().enumerate() {
        v.push(Row::new(RowId::Folder(s, i), short_path(d), Value::Text("Remove".into()), format!("{d}\n\nPress A to remove this folder.")));
    }
    v.push(Row::new(
        RowId::Import(s),
        "Use Emulator's Game List",
        Value::Toggle(sc.import_from_emulator),
        "Also show games the emulator already has in its own library, so you don't need to add those folders here.",
    ));
    let defaults = emu.as_ref().map(|e| s.default_args(e).join(" ")).unwrap_or_default();
    v.push(Row::new(
        RowId::Args(s),
        "Launch Options",
        Value::Text(if sc.args.is_empty() { "Default".into() } else { sc.args.join(" ") }),
        format!("Advanced. Command-line options passed to the emulator. Leave empty to use the defaults ({defaults})."),
    ));
    v.push(Row::note(format!("{count} game(s) found.")));
}

fn short_path(p: &str) -> String {
    let parts: Vec<&str> = p.split(['\\', '/']).filter(|s| !s.is_empty()).collect();
    match parts.len() {
        0 => p.to_string(),
        1 => parts[0].to_string(),
        n => format!("…\\{}\\{}", parts[n - 2], parts[n - 1]),
    }
}

fn first_selectable(rows: &[Row]) -> usize {
    rows.iter().position(|r| r.selectable()).unwrap_or(0)
}

// ---------------------------------------------------------------- input
impl App {
    pub fn menu_input(&mut self, p: Pad, frame: &eframe::Frame, ctx: &egui::Context) {
        let Some(m) = self.menu.as_ref() else { return };
        if m.closing.is_some() {
            return;
        }
        if let Some(e) = m.edit.as_ref() {
            if let Some(id) = e.widget {
                if matches!(p, Pad::A | Pad::Start | Pad::B) {
                    ctx.memory_mut(|mem| mem.surrender_focus(id));
                }
            }
            match p {
                Pad::A | Pad::Start => self.commit_edit(),
                Pad::B => self.menu.as_mut().unwrap().edit = None,
                _ => {}
            }
            return;
        }
        let rows = rows(self, m);
        let m = self.menu.as_mut().unwrap();
        let ncat = m.ncats();
        if !m.in_list {
            match p {
                Pad::Up if m.cat > 0 => {
                    m.cat -= 1;
                    self.sounds.play("move");
                }
                Pad::Down if m.cat + 1 < ncat => {
                    m.cat += 1;
                    self.sounds.play("move");
                }
                Pad::Right | Pad::A => {
                    m.in_list = true;
                    let r = rows_for(self);
                    let m = self.menu.as_mut().unwrap();
                    m.row = first_selectable(&r);
                    m.scroll = 0;
                    self.sounds.play("select");
                }
                Pad::B | Pad::Start => {
                    self.close_menu();
                    self.sounds.play("back");
                }
                _ => {}
            }
            return;
        }
        match p {
            Pad::Up => {
                if let Some(i) = (0..m.row).rev().find(|&i| rows[i].selectable()) {
                    m.row = i;
                    self.sounds.play("move");
                }
            }
            Pad::Down => {
                if let Some(i) = (m.row + 1..rows.len()).find(|&i| rows[i].selectable()) {
                    m.row = i;
                    self.sounds.play("move");
                }
            }
            Pad::Left | Pad::B if m.kind == Kind::Settings => {
                if let Some(c) = m.console.take() {
                    // Back from a console page to the console list.
                    m.row = systems::ALL.iter().position(|x| *x == c).unwrap_or(0);
                    m.bar = m.row as f32;
                } else {
                    m.in_list = false;
                }
                self.sounds.play("back");
            }
            Pad::B if m.kind == Kind::Setup && m.cat > 0 => self.setup_step(-1),
            Pad::Start if m.kind == Kind::Settings => {
                self.close_menu();
                self.sounds.play("back");
            }
            Pad::A => {
                if let Some(r) = rows.get(m.row) {
                    let id = r.id;
                    self.sounds.play("select");
                    self.activate_row(id, frame, ctx);
                }
            }
            _ => {}
        }
    }

    fn setup_step(&mut self, d: i32) {
        let Some(m) = self.menu.as_mut() else { return };
        m.cat = (m.cat as i32 + d).clamp(0, SETUP_STEPS.len() as i32 - 1) as usize;
        let r = rows_for(self);
        let m = self.menu.as_mut().unwrap();
        // Land on Continue so a controller user can just keep pressing A.
        m.row = r.iter().position(|r| matches!(r.id, RowId::Continue | RowId::Finish)).unwrap_or_else(|| first_selectable(&r));
        m.scroll = 0;
        self.sounds.play(if d > 0 { "select" } else { "back" });
    }

    pub fn activate_row(&mut self, id: RowId, frame: &eframe::Frame, ctx: &egui::Context) {
        match id {
            RowId::Gamertag => self.start_edit(id, self.cfg.gamertag.clone()),
            RowId::RaUser => self.start_edit(id, self.cfg.ra_user.clone()),
            RowId::RaPassword => self.start_edit(id, self.ra_password.clone()),
            RowId::RaSignIn => {
                if self.cfg.ra_user.trim().is_empty() {
                    self.toast("Type your username first");
                } else if self.ra_password.is_empty() {
                    self.toast("Type your password first");
                } else {
                    let pw = std::mem::take(&mut self.ra_password);
                    self.ra_login(pw);
                }
            }
            RowId::OpenConsole(s) => {
                if let Some(m) = self.menu.as_mut() {
                    m.console = Some(s);
                    m.row = 0;
                    m.bar = 0.0;
                    m.scroll = 0;
                }
            }
            RowId::EmuOpt(s, i) => {
                if let Some(e) = self.cfg.emulator(s) {
                    if let Some(o) = crate::emusettings::options(s, &e).get(i) {
                        match crate::emusettings::cycle(o) {
                            Ok(v) => self.toast(format!("{}: {v}", o.label)),
                            Err(err) => self.toast(format!("Couldn't save: {err}")),
                        }
                    }
                }
            }
            RowId::OpenEmu(s) => {
                self.close_menu();
                self.launching = Some((Instant::now(), None, Some(s)));
            }
            RowId::Gamerpic => self.open_pic_picker(),
            RowId::CustomPic => self.choose_custom_picture(frame, ctx),
            RowId::ShowVersion => {
                self.cfg.show_version = !self.cfg.show_version;
                self.cfg.save();
            }
            RowId::FrameRate => {
                const STEPS: [u32; 6] = [0, 240, 144, 120, 60, 30];
                let i = STEPS.iter().position(|f| *f == self.cfg.frame_rate).unwrap_or(0);
                self.cfg.frame_rate = STEPS[(i + 1) % STEPS.len()];
                self.cfg.save();
            }
            RowId::EmulatorExe(s) => {
                let mut d = rfd::FileDialog::new().set_title(format!("Choose the {} program ({})", s.emu_label(), s.emulator_hint())).set_parent(frame);
                if cfg!(windows) {
                    d = d.add_filter("Programs", &["exe"]);
                }
                if let Some(p) = d.pick_file() {
                    self.cfg.sys_mut(s).emulator = p.to_string_lossy().to_string();
                    self.cfg.save();
                    self.rescan();
                    self.toast(format!("{} set", s.emu_label()));
                }
            }
            RowId::EmulatorFolder(s) => {
                if let Some(dir) = rfd::FileDialog::new().set_title(format!("Choose the folder {} is in", s.emu_label())).set_parent(frame).pick_folder() {
                    match s.find_in(&dir) {
                        Some(p) => {
                            self.cfg.sys_mut(s).emulator = p.to_string_lossy().to_string();
                            self.cfg.save();
                            self.rescan();
                            self.toast(format!("Found {}", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
                        }
                        None => self.toast(format!("No {} found in that folder", s.emulator_hint())),
                    }
                }
            }
            RowId::EmulatorClear(s) => {
                self.cfg.sys_mut(s).emulator.clear();
                self.cfg.save();
                self.rescan();
            }
            RowId::RaRefresh => self.ra_refresh(),
            RowId::RaSignOut => self.ra_logout(),
            RowId::Backend(s) => {
                let c = self.cfg.sys_mut(s);
                c.backend = match c.backend.as_str() {
                    "auto" => "emulator",
                    "emulator" => "core",
                    _ => "auto",
                }
                .to_string();
                self.cfg.save();
                self.rebuild();
            }
            RowId::InstallCore(s) => {
                if crate::runtime::installed(s) {
                    self.toast("Already installed");
                } else {
                    self.install_core(s, None, true);
                }
            }
            RowId::Theme => {
                let i = scene::THEMES.iter().position(|t| *t == self.cfg.theme).unwrap_or(0);
                self.set_theme(scene::THEMES[(i + 1) % scene::THEMES.len()]);
            }
            RowId::Args(s) => self.start_edit(id, self.cfg.sys(s).args.join(" ")),
            RowId::Fullscreen => {
                self.cfg.fullscreen = !self.cfg.fullscreen;
                self.fullscreen = self.cfg.fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
                self.cfg.save();
            }
            RowId::Sounds => {
                self.cfg.sounds = !self.cfg.sounds;
                self.sounds = crate::sound::Sounds::new(theme_dir().join("sounds"), self.cfg.sounds);
                self.cfg.save();
            }
            RowId::Minimize => {
                self.cfg.minimize_while_playing = !self.cfg.minimize_while_playing;
                self.cfg.save();
            }
            RowId::Rescan => {
                self.rescan();
                self.toast(format!("{} games found", self.games.len()));
            }
            RowId::RunSetup => self.menu = Some(Menu::new(Kind::Setup)),
            RowId::OpenFile => crate::open_path(&config_path()),
            RowId::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            RowId::Emulator(s) => self.emulator_dialog(s),
            RowId::Import(s) => {
                let c = self.cfg.sys_mut(s);
                c.import_from_emulator = !c.import_from_emulator;
                self.cfg.save();
                self.rescan();
            }
            RowId::Folder(s, i) => {
                let c = self.cfg.sys_mut(s);
                if i < c.game_dirs.len() {
                    c.game_dirs.remove(i);
                }
                self.cfg.save();
                self.rescan();
                let r = rows_for(self);
                if let Some(m) = self.menu.as_mut() {
                    if !r.get(m.row).map(|r| r.selectable()).unwrap_or(false) {
                        m.row = (0..r.len()).rev().find(|&i| r[i].selectable()).unwrap_or(0);
                    }
                }
            }
            RowId::AddFolder(s) => self.add_folder(s, frame),
            RowId::AutoCovers => {
                self.cfg.auto_covers = !self.cfg.auto_covers;
                self.cfg.save();
                if self.cfg.auto_covers {
                    self.queue_auto_covers();
                }
            }
            RowId::FetchCovers => {
                self.tried.clear();
                let _ = std::fs::remove_file(theme_dir().join("cache").join("cover-tried.txt"));
                self.queue_auto_covers_forced();
                self.toast("Looking for missing covers…");
            }
            RowId::ResetCovers => {
                let _ = std::fs::remove_dir_all(theme_dir().join("cache").join("covers"));
                let _ = std::fs::remove_file(theme_dir().join("cache").join("cover-tried.txt"));
                self.tried.clear();
                self.rescan();
                self.queue_auto_covers_forced();
                self.toast("Downloaded covers cleared, searching again");
            }
            RowId::Continue => self.setup_step(1),
            RowId::Back => self.setup_step(-1),
            RowId::Finish => {
                self.cfg.setup_done = true;
                self.cfg.save();
                self.rescan();
                // The setup fades out over the start of the dashboard intro.
                self.close_menu();
                self.start_intro();
            }
            RowId::None => {}
        }
    }

    pub fn add_folder(&mut self, s: System, frame: &eframe::Frame) {
        if let Some(p) = rfd::FileDialog::new().set_title(format!("Choose a {} game folder", s.short())).set_parent(frame).pick_folder() {
            let st = p.to_string_lossy().to_string();
            let c = self.cfg.sys_mut(s);
            if !c.game_dirs.contains(&st) {
                c.game_dirs.push(st);
            }
            self.cfg.save();
            self.rescan();
            let n = self.games.iter().filter(|g| g.system == s).count();
            self.toast(format!("{n} {} games found", s.short()));
        }
    }

    fn start_edit(&mut self, id: RowId, text: String) {
        if let Some(m) = self.menu.as_mut() {
            m.edit = Some(Edit { target: id, text, focus_set: false, widget: None });
        }
    }

    pub fn commit_edit(&mut self) {
        let Some(e) = self.menu.as_mut().and_then(|m| m.edit.take()) else { return };
        match e.target {
            RowId::Gamertag => {
                let t = e.text.trim();
                if !t.is_empty() {
                    self.cfg.gamertag = t.chars().take(24).collect();
                }
            }
            RowId::Args(s) => {
                self.cfg.sys_mut(s).args = split_args(&e.text);
            }
            RowId::RaUser => {
                let u = e.text.trim().to_string();
                if u != self.cfg.ra_user {
                    self.cfg.ra_user = u;
                    self.ra_logout();
                }
                self.cfg.save();
            }
            RowId::RaPassword => {
                self.ra_password = e.text;
                // Pressing Enter in the password box signs in straight away.
                if !self.ra_password.is_empty() && !self.cfg.ra_user.trim().is_empty() {
                    let pw = std::mem::take(&mut self.ra_password);
                    self.ra_login(pw);
                }
            }
            _ => {}
        }
        self.cfg.save();
    }
}

fn rows_for(app: &App) -> Vec<Row> {
    app.menu.as_ref().map(|m| rows(app, m)).unwrap_or_default()
}

/// Splits on spaces, keeping "quoted parts" together.
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q = false;
    for c in s.chars() {
        match c {
            '"' => q = !q,
            ' ' if !q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

// ---------------------------------------------------------------- drawing

// ---------------------------------------------------------------- drawing helpers

pub struct Frame360 {
    pub u: f32,
    pub panel: Rect,
    pub list: Rect,
    pub right: Rect,
    pub row_h: f32,
}

/// Background, big title and the slate panel. Returns the layout.
/// Page chrome (title + slate panel). The background lives on its own layer underneath.
pub fn frame360(p: &egui::Painter, _app: &App, screen: Rect, title: &str, subtitle: Option<&str>, a: f32, _bg: bool) -> Frame360 {
    let (w, h) = (screen.width(), screen.height());
    let u = (h / 720.0).min(w / 1280.0);
    p.rect_filled(screen.expand(screen.width()), 0.0, Color32::from_black_alpha((40.0 * a) as u8));
    text_shadow(p, pos2(96.0 * u, 67.0 * u), Align2::LEFT_CENTER, title, 32.0 * u, Color32::WHITE.gamma_multiply(a));
    if let Some(s) = subtitle {
        text_shadow(p, pos2(96.0 * u, 96.0 * u), Align2::LEFT_CENTER, s, 17.0 * u, Color32::from_white_alpha(220).gamma_multiply(a));
    }
    let pw = 890.0 * u;
    let slide = (1.0 - a).powi(3) * 28.0 * u;
    let panel = Rect::from_min_size(pos2(w * 0.5 - pw * 0.5, 112.0 * u + slide), vec2(pw, 480.0 * u));
    let rad = CornerRadius::same((6.0 * u) as u8);
    p.rect_filled(panel.translate(vec2(0.0, 4.0 * u)), rad, Color32::from_black_alpha((50.0 * a) as u8));
    p.rect_filled(panel, rad, Color32::from_rgba_unmultiplied(66, 82, 100, (232.0 * a) as u8));
    // Soft top sheen.
    let sheen = Rect::from_min_max(panel.min, pos2(panel.right(), panel.top() + 120.0 * u));
    let mut m = egui::Mesh::default();
    m.colored_vertex(sheen.left_top(), Color32::from_white_alpha((16.0 * a) as u8));
    m.colored_vertex(sheen.right_top(), Color32::from_white_alpha((16.0 * a) as u8));
    m.colored_vertex(sheen.left_bottom(), Color32::TRANSPARENT);
    m.colored_vertex(sheen.right_bottom(), Color32::TRANSPARENT);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    p.add(egui::Shape::mesh(m));
    p.rect_stroke(panel, rad, Stroke::new(1.0, Color32::from_white_alpha((30.0 * a) as u8)), StrokeKind::Inside);
    let list = Rect::from_min_max(panel.min + vec2(18.0 * u, 14.0 * u), pos2(panel.left() + 452.0 * u, panel.bottom() - 34.0 * u));
    let right = Rect::from_min_max(pos2(panel.left() + 480.0 * u, panel.top() + 22.0 * u), panel.max - vec2(26.0 * u, 22.0 * u));
    Frame360 { u, panel, list, right, row_h: 46.0 * u }
}

thread_local! {
    /// When set, rows don't draw their own bar (the caller draws an animated one).
    static BAR_EXTERNAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The green selection bar at a (possibly fractional) visible row.
fn sel_bar(p: &egui::Painter, f: &Frame360, pos: f32, a: f32) {
    let u = f.u;
    let r = Rect::from_min_size(pos2(f.list.left(), f.list.top() + pos * f.row_h), vec2(f.list.width(), f.row_h - 4.0 * u));
    let bar = r.shrink2(vec2(0.0, 2.0 * u));
    let mut m = egui::Mesh::default();
    let top = Color32::from_rgb(150, 206, 62).gamma_multiply(a);
    let bot = Color32::from_rgb(78, 146, 22).gamma_multiply(a);
    m.colored_vertex(bar.left_top(), top);
    m.colored_vertex(bar.right_top(), top);
    m.colored_vertex(bar.left_bottom(), bot);
    m.colored_vertex(bar.right_bottom(), bot);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    p.add(egui::Shape::mesh(m));
    p.rect_stroke(bar, 2.0 * u, Stroke::new(1.0, Color32::from_white_alpha((70.0 * a) as u8)), StrokeKind::Inside);
}

/// One list row: optional icon, label, separator, green bar when selected.
fn list_row(p: &egui::Painter, f: &Frame360, i_vis: usize, label: &str, icon: Option<Icon>, selected: bool, enabled: bool, a: f32, accent_label: bool) -> Rect {
    let u = f.u;
    let r = Rect::from_min_size(pos2(f.list.left(), f.list.top() + i_vis as f32 * f.row_h), vec2(f.list.width(), f.row_h - 4.0 * u));
    if selected && !BAR_EXTERNAL.with(|b| b.get()) {
        sel_bar(p, f, i_vis as f32, a);
    }
    p.line_segment(
        [pos2(r.left() + 4.0 * u, r.bottom() + 2.0 * u), pos2(r.right() - 4.0 * u, r.bottom() + 2.0 * u)],
        Stroke::new(1.0, Color32::from_white_alpha((26.0 * a) as u8)),
    );
    let mut x = r.left() + 14.0 * u;
    let col = if !enabled {
        Color32::from_white_alpha(90)
    } else if selected {
        Color32::WHITE
    } else if accent_label {
        Color32::from_rgb(180, 228, 110)
    } else {
        Color32::from_rgb(214, 222, 230)
    }
    .gamma_multiply(a);
    if let Some(ic) = icon {
        scene::icon_pub(p, ic, pos2(x + 11.0 * u, r.center().y), 11.0 * u, col);
        x += 34.0 * u;
    }
    let font = FontId::proportional(21.0 * u);
    let t = fit(p, label, &font, r.right() - x - 10.0 * u);
    if selected {
        p.text(pos2(x + 1.0, r.center().y + 1.5), Align2::LEFT_CENTER, &t, font.clone(), Color32::from_black_alpha((70.0 * a) as u8));
    }
    p.text(pos2(x, r.center().y), Align2::LEFT_CENTER, t, font, col);
    r
}

fn arrows(p: &egui::Painter, f: &Frame360, up: bool, down: bool, a: f32) {
    let u = f.u;
    let c = pos2(f.list.center().x, f.panel.bottom() - 17.0 * u);
    let col = Color32::from_white_alpha((180.0 * a) as u8);
    if up {
        p.add(egui::Shape::convex_polygon(vec![c + vec2(-7.0 * u - 14.0 * u, 3.0 * u), c + vec2(7.0 * u - 14.0 * u, 3.0 * u), c + vec2(-14.0 * u, -4.0 * u)], col, Stroke::NONE));
    }
    if down {
        p.add(egui::Shape::convex_polygon(vec![c + vec2(-7.0 * u + 14.0 * u, -3.0 * u), c + vec2(7.0 * u + 14.0 * u, -3.0 * u), c + vec2(14.0 * u, 4.0 * u)], col, Stroke::NONE));
    }
}

/// Wrapped text block; returns the height used.
fn para(p: &egui::Painter, pos: Pos2, text: &str, size: f32, width: f32, col: Color32) -> f32 {
    let g = p.layout(text.to_string(), FontId::proportional(size), col, width);
    let hgt = g.size().y;
    p.galley(pos, g, col);
    hgt
}

/// Springs the selection bar toward `target` (frame-rate independent).
fn animate_bar(app: &mut App, target: f32) -> f32 {
    let m = app.menu.as_mut().unwrap();
    let dt = m.last_frame.elapsed().as_secs_f32().min(0.05);
    m.last_frame = Instant::now();
    if m.opened.elapsed().as_secs_f32() < 0.05 || (target - m.bar).abs() > 6.0 {
        m.bar = target;
        m.bar_vel = 0.0;
    }
    let w = 26.0f32;
    let steps = (dt * 480.0).ceil().max(1.0) as usize;
    let h = dt / steps as f32;
    for _ in 0..steps {
        let acc = w * w * (target - m.bar) - 2.0 * w * m.bar_vel;
        m.bar_vel += acc * h;
        m.bar += m.bar_vel * h;
    }
    m.bar
}

// ---------------------------------------------------------------- settings / setup

/// A key that changes whenever a different page of the menu is showing. The number of ':'
/// is the depth (top level < category < console page), used to pick the zoom direction.
pub fn page_key(m: &Menu) -> String {
    match (m.kind, m.in_list, m.console) {
        (Kind::Setup, _, _) => format!("menu:setup{}", ":".repeat(m.cat + 1)),
        (Kind::Settings, false, _) => "menu:settings".into(),
        (Kind::Settings, true, None) => format!("menu:settings:{}", m.cat),
        (Kind::Settings, true, Some(c)) => format!("menu:settings:{}:{}", m.cat, c.key()),
    }
}

pub fn draw(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(m) = app.menu.as_ref() else { return };
    let rows = rows(app, m);
    if m.in_list && !rows.get(m.row).map(|r| r.selectable()).unwrap_or(false) {
        let fs = (m.row..rows.len()).find(|&i| rows[i].selectable()).unwrap_or_else(|| first_selectable(&rows));
        app.menu.as_mut().unwrap().row = fs;
    }
    let m = app.menu.as_ref().unwrap();
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let closing = m.closing.map(|c| c.elapsed().as_secs_f32());
    let a = (m.opened.elapsed().as_secs_f32() / 0.22).min(1.0) * closing.map(|c| (1.0 - c / CLOSE_TIME).clamp(0.0, 1.0).powi(2)).unwrap_or(1.0);
    let (title, subtitle) = match (m.kind, m.in_list) {
        (Kind::Settings, false) => ("Settings".to_string(), None),
        (Kind::Settings, true) => match m.console {
            Some(c) => (c.channel().to_string(), Some("Settings  ·  Consoles".to_string())),
            None => (settings_cats()[m.cat].0.clone(), Some("Settings".to_string())),
        },
        (Kind::Setup, _) => ("Quick Setup".to_string(), Some(format!("Step {} of {}  ·  {}", m.cat + 1, SETUP_STEPS.len(), SETUP_STEPS[m.cat]))),
    };
    let f = frame360(&p, app, screen, &title, subtitle.as_deref(), a, closing.is_none());
    let u = f.u;
    let max_rows = ((f.list.height()) / f.row_h).floor() as usize;
    let text_col = Color32::from_rgb(226, 232, 238).gamma_multiply(a);
    let dim_col = Color32::from_rgb(176, 188, 200).gamma_multiply(a);
    let mut edit_rect: Option<Rect> = None;

    if m.kind == Kind::Settings && !m.in_list {
        // Top level: categories with icons, description on the right.
        let cats = settings_cats();
        let top = m.cat.saturating_sub(max_rows.saturating_sub(1));
        let target = (m.cat - top) as f32;
        let bar = animate_bar(app, target);
        let m = app.menu.as_ref().unwrap();
        sel_bar(&p, &f, bar, a);
        BAR_EXTERNAL.with(|b| b.set(true));
        for (i, (name, ic, _)) in cats.iter().enumerate().skip(top).take(max_rows) {
            list_row(&p, &f, i - top, name, Some(*ic), i == m.cat, true, a, false);
        }
        arrows(&p, &f, top > 0, top + max_rows < cats.len(), a);
        BAR_EXTERNAL.with(|b| b.set(false));
        para(&p, f.right.min, &cats[m.cat].2, 21.0 * u, f.right.width(), text_col);
    } else {
        // Inside a category: selectable rows in the list, notes on the right.
        let vis: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].selectable()).collect();
        let sel_vis = vis.iter().position(|&i| i == m.row).unwrap_or(0);
        let mut scroll = m.scroll.min(vis.len().saturating_sub(max_rows));
        if sel_vis < scroll {
            scroll = sel_vis;
        }
        if sel_vis >= scroll + max_rows {
            scroll = sel_vis + 1 - max_rows;
        }
        let bar = animate_bar(app, (sel_vis - scroll) as f32);
        let m = app.menu.as_ref().unwrap();
        sel_bar(&p, &f, bar, a);
        BAR_EXTERNAL.with(|b| b.set(true));
        for (k, &ri) in vis.iter().enumerate().skip(scroll).take(max_rows) {
            let r = &rows[ri];
            let accent = matches!(r.id, RowId::Continue | RowId::Finish);
            let rr = list_row(&p, &f, k - scroll, &r.label, None, ri == m.row, r.id != RowId::None, a, accent);
            // Status dot / toggle state at the right edge of the row.
            match &r.value {
                Value::Toggle(on) => {
                    let t = if *on { "On" } else { "Off" };
                    p.text(pos2(rr.right() - 14.0 * u, rr.center().y), Align2::RIGHT_CENTER, t, FontId::proportional(18.0 * u), if ri == m.row { Color32::WHITE.gamma_multiply(a) } else { dim_col });
                }
                Value::Status(ok, _) => {
                    p.circle_filled(pos2(rr.right() - 20.0 * u, rr.center().y), 6.0 * u, if *ok { Color32::from_rgb(140, 220, 70) } else { Color32::from_rgb(235, 160, 50) }.gamma_multiply(a));
                }
                Value::Text(t) => {
                    // The value lives in the row; editing happens right here too.
                    let label_w = p.layout_no_wrap(r.label.clone(), FontId::proportional(21.0 * u), Color32::WHITE).size().x;
                    let vx = rr.left() + 14.0 * u + label_w + 26.0 * u;
                    let editing = m.edit.as_ref().map(|e| e.target == r.id).unwrap_or(false);
                    if editing {
                        edit_rect = Some(Rect::from_min_max(pos2(vx.max(rr.left() + rr.width() * 0.3), rr.top() + 5.0 * u), pos2(rr.right() - 8.0 * u, rr.bottom() - 5.0 * u)));
                    } else if !t.is_empty() {
                        let font = FontId::proportional(18.0 * u);
                        let shown = fit(&p, t, &font, rr.right() - 16.0 * u - vx);
                        p.text(pos2(rr.right() - 14.0 * u, rr.center().y), Align2::RIGHT_CENTER, shown, font, if ri == m.row { Color32::from_white_alpha(235).gamma_multiply(a) } else { dim_col });
                    }
                }
                _ => {}
            }
        }
        BAR_EXTERNAL.with(|b| b.set(false));
        arrows(&p, &f, scroll > 0, scroll + max_rows < vis.len(), a);
        let m2 = app.menu.as_mut().unwrap();
        m2.scroll = scroll;
        let m = app.menu.as_ref().unwrap();

        // Right column.
        let mut y = f.right.top();
        let width = f.right.width();
        if let Some(r) = rows.get(m.row) {
            // Only statuses (like an emulator's path) need the extra "Current Setting" block;
            // text and on/off values are already shown in the row.
            let current = match &r.value {
                Value::Status(_, t) if !t.is_empty() => Some(t.clone()),
                _ => None,
            };
            if let Some(c) = current {
                p.text(pos2(f.right.left(), y), Align2::LEFT_TOP, "Current Setting", FontId::proportional(21.0 * u), Color32::WHITE.gamma_multiply(a));
                y += 30.0 * u;
                y += para(&p, pos2(f.right.left() + 4.0 * u, y), &c, 19.0 * u, width - 4.0 * u, dim_col) + 26.0 * u;
            }
            if !r.desc.is_empty() {
                y += para(&p, pos2(f.right.left(), y), &r.desc, 20.0 * u, width, text_col) + 22.0 * u;
            }
        }
        for n in rows.iter().filter(|r| matches!(r.value, Value::Note)) {
            if y > f.right.bottom() - 30.0 * u {
                break;
            }
            y += para(&p, pos2(f.right.left(), y), &n.label, 17.0 * u, width, dim_col) + 12.0 * u;
        }
        if rows.iter().any(|r| r.id == RowId::Gamerpic) {
            let side = 150.0 * u;
            let r = Rect::from_min_size(pos2(f.right.left(), (y + 10.0 * u).max(f.right.bottom() - side - 40.0 * u)), vec2(side, side));
            crate::profile::draw_current(app, &p, r, a);
            p.rect_stroke(r, 4.0 * u, Stroke::new(1.5 * u, Color32::from_white_alpha((120.0 * a) as u8)), StrokeKind::Inside);
            p.text(
                pos2(r.right() + 16.0 * u, r.bottom() - 12.0 * u),
                Align2::LEFT_CENTER,
                crate::profile::current_name(app),
                FontId::proportional(19.0 * u),
                Color32::WHITE.gamma_multiply(a),
            );
        }
    }

    // Setup progress dots under the panel title area.
    let m = app.menu.as_ref().unwrap();
    if m.kind == Kind::Setup {
        for i in 0..SETUP_STEPS.len() {
            let c = pos2(f.panel.right() - 20.0 * u - (SETUP_STEPS.len() - 1 - i) as f32 * 22.0 * u, f.panel.top() - 18.0 * u);
            let on = i <= m.cat;
            p.circle_filled(c, 6.0 * u, if on { Color32::WHITE } else { Color32::from_white_alpha(90) }.gamma_multiply(a));
        }
    }

    let editing = m.edit.is_some();
    let signing_in = m.edit.as_ref().map(|e| e.target == RowId::RaPassword).unwrap_or(false);
    let hints: Vec<(&str, &str, Color32)> = if editing {
        vec![("A", if signing_in { "Sign In (Enter)" } else { "Save (Enter)" }, BTN_A), ("B", "Cancel (Esc)", BTN_B)]
    } else {
        vec![("A", "Select", BTN_A), ("B", "Back", BTN_B)]
    };
    hint_bar(&p, &hints, 100.0 * u, 650.0 * u, u, a);

    // Text box for editing.
    if let Some(er) = edit_rect {
        let m = app.menu.as_mut().unwrap();
        if let Some(e) = m.edit.as_mut() {
            let resp = egui::Area::new(egui::Id::unique("menu_edit"))
                .fixed_pos(er.min)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.add_sized(
                        er.size(),
                        egui::TextEdit::singleline(&mut e.text)
                            .password(e.target == RowId::RaPassword)
                            .font(FontId::proportional(19.0 * u))
                            .text_color(Color32::from_rgb(20, 24, 28))
                            .background_color(Color32::from_rgb(240, 244, 236))
                            .margin(vec2(10.0 * u, 5.0 * u)),
                    )
                })
                .inner;
            e.widget = Some(resp.id);
            let mut finish = None;
            if !e.focus_set {
                resp.request_focus();
                e.focus_set = true;
            } else if resp.lost_focus() {
                // Enter / Esc are eaten by the text box itself, so act on it losing focus.
                finish = Some(!ctx.input(|i| i.key_pressed(egui::Key::Escape)));
            }
            match finish {
                Some(true) => app.commit_edit(),
                Some(false) => app.menu.as_mut().unwrap().edit = None,
                None => {}
            }
        }
    }
}

// ---------------------------------------------------------------- cover picker

pub struct Picker {
    pub game: usize,
    pub req: u64,
    /// (label, texture, bytes). Index 0 is "Automatic".
    pub items: Vec<(String, Option<egui::TextureHandle>, Vec<u8>)>,
    pub sel: usize,
    pub done: bool,
    pub opened: Instant,
    pub scroll: f32,
}

impl App {
    pub fn open_picker(&mut self, gi: usize) {
        self.picker_req += 1;
        let req = self.picker_req;
        let g = self.games[gi].as_ref();
        let _ = self.cover_pri_tx.send(covers::Job::Candidates(req, g));
        self.picker = Some(Picker {
            game: gi,
            req,
            items: vec![("Automatic".into(), None, vec![])],
            sel: 0,
            done: false,
            opened: Instant::now(),
            scroll: 0.0,
        });
        self.sounds.play("select");
    }

    pub fn picker_candidates(&mut self, ctx: &egui::Context, req: u64, list: Vec<(String, Vec<u8>)>) {
        let Some(pk) = self.picker.as_mut().filter(|p| p.req == req) else { return };
        for (i, (label, bytes)) in list.into_iter().enumerate() {
            if pk.items.len() > i + 1 {
                continue;
            }
            let tex = image::load_from_memory(&bytes).ok().map(|img| {
                let img = img.thumbnail(512, 512).to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                ctx.load_texture(format!("cand{req}_{i}"), ci, egui::TextureOptions::LINEAR)
            });
            pk.items.push((label, tex, bytes));
        }
    }

    pub fn picker_input(&mut self, p: Pad) {
        let Some(pk) = self.picker.as_mut() else { return };
        match p {
            Pad::Left if pk.sel > 0 => {
                pk.sel -= 1;
                self.sounds.play("move");
            }
            Pad::Right if pk.sel + 1 < pk.items.len() => {
                pk.sel += 1;
                self.sounds.play("move");
            }
            Pad::B | Pad::Y => {
                self.picker = None;
                self.sounds.play("back");
            }
            Pad::A => {
                let gi = pk.game;
                let g = self.games[gi].as_ref();
                let dir = covers::manual_dir(&theme_dir(), g.system);
                if pk.sel == 0 {
                    covers::remove_image(&dir, &g.key());
                } else {
                    let bytes = pk.items[pk.sel].2.clone();
                    covers::save_image(&dir, &g.key(), &bytes);
                }
                self.picker = None;
                self.refresh_cover(gi);
                self.sounds.play("select");
                self.toast("Cover updated");
            }
            _ => {}
        }
    }
}



pub fn draw_picker(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(pk) = app.picker.as_ref() else { return };
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let a = (pk.opened.elapsed().as_secs_f32() / 0.22).min(1.0);
    let name = app.games.get(pk.game).map(|g| g.name.clone()).unwrap_or_default();
    let f = frame360(&p, app, screen, "Choose a Cover", Some(&name), a, true);
    let u = f.u;
    let pk = app.picker.as_mut().unwrap();
    let inner = f.panel.shrink2(vec2(24.0 * u, 20.0 * u));
    let card_h = 330.0 * u;
    let gap = 26.0 * u;
    let widths: Vec<f32> = pk
        .items
        .iter()
        .map(|(_, t, _)| match t {
            Some(t) => {
                let [tw, th] = t.size();
                (card_h * tw as f32 / th.max(1) as f32).clamp(card_h * 0.55, card_h * 1.8)
            }
            None => card_h * 0.7,
        })
        .collect();
    let sel_x: f32 = widths.iter().take(pk.sel).map(|w| w + gap).sum::<f32>() + widths[pk.sel] * 0.5;
    let target = (sel_x - inner.width() * 0.5).max(0.0);
    pk.scroll += (target - pk.scroll) * 0.25;
    let clip = p.with_clip_rect(f.panel.shrink(2.0));
    let mut x = inner.left() - pk.scroll;
    let top = inner.top() + 10.0 * u;
    for (i, (label, tex, _)) in pk.items.iter().enumerate() {
        let cw = widths[i];
        let r = Rect::from_min_size(pos2(x, top), vec2(cw, card_h));
        x += cw + gap;
        if r.right() < f.panel.left() || r.left() > f.panel.right() {
            continue;
        }
        let sel = i == pk.sel;
        if sel {
            clip.rect_filled(r.expand(6.0 * u), CornerRadius::same((5.0 * u) as u8), Color32::from_rgb(120, 190, 40).gamma_multiply(a));
        }
        match tex {
            Some(t) => {
                clip.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), if sel { Color32::WHITE } else { Color32::from_gray(150) }.gamma_multiply(a));
            }
            None => {
                clip.rect_filled(r, 4.0 * u, Color32::from_white_alpha(18).gamma_multiply(a));
                scene::icon_pub(&clip, Icon::Refresh, r.center() - vec2(0.0, 24.0 * u), 36.0 * u, Color32::from_white_alpha(210).gamma_multiply(a));
                clip.text(r.center() + vec2(0.0, 40.0 * u), Align2::CENTER_CENTER, "Automatic", FontId::proportional(19.0 * u), Color32::WHITE.gamma_multiply(a));
            }
        }
        let font = FontId::proportional(17.0 * u);
        let t = fit(&clip, label, &font, cw.max(200.0 * u));
        clip.text(pos2(r.center().x, r.bottom() + 26.0 * u), Align2::CENTER_CENTER, t, font, Color32::from_white_alpha(if sel { 240 } else { 150 }).gamma_multiply(a));
    }
    let status = if !pk.done {
        let dots = ".".repeat(1 + (pk.opened.elapsed().as_millis() / 400 % 3) as usize);
        format!("Searching online{dots}")
    } else if pk.items.len() == 1 {
        "Nothing found online. You can put your own image in theme\\covers\\.".into()
    } else {
        format!("{} cover(s) found", pk.items.len() - 1)
    };
    p.text(pos2(f.panel.left() + 24.0 * u, f.panel.bottom() - 24.0 * u), Align2::LEFT_CENTER, status, FontId::proportional(17.0 * u), Color32::from_white_alpha(200).gamma_multiply(a));
    hint_bar(&p, &[("A", "Use This Cover", BTN_A), ("B", "Back", BTN_B)], 100.0 * u, 650.0 * u, u, a);
}

pub fn theme_cover_dirs(s: System) -> crate::library::CoverDirs {
    let t = theme_dir();
    crate::library::CoverDirs { manual: vec![covers::manual_dir(&t, s), t.join("covers")], emulator: vec![], auto: covers::auto_dir(&t, s) }
}

// ---------------------------------------------------------------- achievements

pub struct AchView {
    pub game: usize,
    pub req: u64,
    pub title: String,
    pub list: Vec<crate::ra::Achievement>,
    pub badges: Vec<Option<egui::TextureHandle>>,
    pub sel: usize,
    pub loading: bool,
    pub opened: Instant,
    pub scroll: usize,
}

impl App {
    pub fn open_achievements(&mut self, gi: usize) {
        if !self.ra_signed_in() {
            self.toast("Sign in to RetroAchievements in Settings > Achievements");
            return;
        }
        let Some(game_id) = self.ra_match.get(&gi).copied() else {
            self.toast("No achievements for this game");
            return;
        };
        self.picker_req += 1;
        let req = self.picker_req;
        if std::env::var("NXE_FAKE_RA").is_err() {
            let _ = self.ra_tx_send(crate::ra::Job::Game { req, game_id, user: self.cfg.ra_user.clone(), token: self.cfg.ra_token.clone() });
        }
        self.ach = Some(AchView {
            game: gi,
            req,
            title: self.games[gi].name.clone(),
            list: vec![],
            badges: vec![],
            sel: 0,
            loading: true,
            opened: Instant::now(),
            scroll: 0,
        });
        self.sounds.play("select");
        if std::env::var("NXE_FAKE_RA").is_ok() {
            let list = (0..14)
                .map(|i| crate::ra::Achievement {
                    title: format!("Test Achievement {}", i + 1),
                    desc: "Defeat the colossus without losing any health on hard mode.".into(),
                    points: [5, 10, 25][i % 3],
                    badge: String::new(),
                    earned: i < 5,
                    hardcore: i < 2,
                })
                .collect();
            let ctx = egui::Context::default();
            self.ach_loaded(&ctx, req, String::new(), list);
        }
    }

    pub fn ach_loaded(&mut self, _ctx: &egui::Context, req: u64, title: String, list: Vec<crate::ra::Achievement>) {
        if let Some(v) = self.ach.as_mut().filter(|v| v.req == req) {
            if !title.is_empty() {
                v.title = title;
            }
            v.badges = vec![None; list.len()];
            v.list = list;
            v.loading = false;
        }
    }

    pub fn ach_badge(&mut self, ctx: &egui::Context, req: u64, idx: usize, bytes: Vec<u8>) {
        if let Some(v) = self.ach.as_mut().filter(|v| v.req == req) {
            if let (Some(slot), Ok(img)) = (v.badges.get_mut(idx), image::load_from_memory(&bytes)) {
                let img = img.to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                *slot = Some(ctx.load_texture(format!("badge{req}_{idx}"), ci, egui::TextureOptions::LINEAR));
            }
        }
    }

    pub fn ach_input(&mut self, p: Pad) {
        let Some(v) = self.ach.as_mut() else { return };
        match p {
            Pad::Up if v.sel > 0 => {
                v.sel -= 1;
                self.sounds.play("move");
            }
            Pad::Down if v.sel + 1 < v.list.len() => {
                v.sel += 1;
                self.sounds.play("move");
            }
            Pad::B | Pad::X => {
                self.ach = None;
                self.sounds.play("back");
            }
            _ => {}
        }
    }
}

pub fn draw_achievements(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(v) = app.ach.as_ref() else { return };
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let a = (v.opened.elapsed().as_secs_f32() / 0.22).min(1.0);
    let earned = v.list.iter().filter(|x| x.earned).count();
    let pts: u32 = v.list.iter().filter(|x| x.earned).map(|x| x.points).sum();
    let total_pts: u32 = v.list.iter().map(|x| x.points).sum();
    let sub = if v.loading { "Loading…".to_string() } else { format!("{earned} of {} unlocked  ·  {pts} of {total_pts} points", v.list.len()) };
    let title = v.title.clone();
    let f = frame360(&p, app, screen, &title, Some(&sub), a, true);
    let u = f.u;
    let v = app.ach.as_mut().unwrap();
    let max_rows = (f.list.height() / f.row_h).floor() as usize;
    if v.sel < v.scroll {
        v.scroll = v.sel;
    }
    if v.sel >= v.scroll + max_rows {
        v.scroll = v.sel + 1 - max_rows;
    }
    for (k, ach) in v.list.iter().enumerate().skip(v.scroll).take(max_rows) {
        let rr = list_row(&p, &f, k - v.scroll, &format!("      {}", ach.title), None, k == v.sel, true, a, false);
        let br = Rect::from_center_size(pos2(rr.left() + 26.0 * u, rr.center().y), vec2(32.0 * u, 32.0 * u));
        match v.badges.get(k).and_then(|b| b.as_ref()) {
            Some(t) => {
                p.image(t.id(), br, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
            }
            None => {
                p.rect_filled(br, 3.0 * u, Color32::from_white_alpha(30).gamma_multiply(a));
            }
        }
    }
    arrows(&p, &f, v.scroll > 0, v.scroll + max_rows < v.list.len(), a);
    if let Some(ach) = v.list.get(v.sel) {
        let mut y = f.right.top();
        let big = Rect::from_min_size(pos2(f.right.left(), y), vec2(96.0 * u, 96.0 * u));
        if let Some(t) = v.badges.get(v.sel).and_then(|b| b.as_ref()) {
            p.image(t.id(), big, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
        } else {
            p.rect_filled(big, 4.0 * u, Color32::from_white_alpha(30).gamma_multiply(a));
        }
        p.text(pos2(big.right() + 18.0 * u, big.top() + 18.0 * u), Align2::LEFT_CENTER, format!("{} points", ach.points), FontId::proportional(20.0 * u), Color32::WHITE.gamma_multiply(a));
        let state = match (ach.earned, ach.hardcore) {
            (true, true) => "Unlocked (hardcore)".to_string(),
            (true, false) => "Unlocked".to_string(),
            (false, _) => "Locked".to_string(),
        };
        p.text(
            pos2(big.right() + 18.0 * u, big.top() + 50.0 * u),
            Align2::LEFT_CENTER,
            state,
            FontId::proportional(17.0 * u),
            if ach.earned { Color32::from_rgb(170, 230, 100) } else { Color32::from_white_alpha(170) }.gamma_multiply(a),
        );
        y = big.bottom() + 20.0 * u;
        y += para(&p, pos2(f.right.left(), y), &ach.title, 22.0 * u, f.right.width(), Color32::WHITE.gamma_multiply(a)) + 10.0 * u;
        para(&p, pos2(f.right.left(), y), &ach.desc, 19.0 * u, f.right.width(), Color32::from_rgb(200, 210, 220).gamma_multiply(a));
    } else if !v.loading {
        para(&p, f.right.min, "No achievements found for this game.", 20.0 * u, f.right.width(), Color32::WHITE.gamma_multiply(a));
    }
    hint_bar(&p, &[("B", "Back", BTN_B)], 100.0 * u, 650.0 * u, u, a);
}


// ---------------------------------------------------------------- small dialog (no emulator, etc.)

#[derive(Clone, Copy, PartialEq)]
pub enum DialogAct {
    DownloadCore(System, usize),
    OpenSettings(System),
    /// Run a settings row action (keeps the settings menu open).
    Row(RowId),
    Close,
}

pub struct Dialog {
    pub title: String,
    pub body: String,
    pub options: Vec<(String, DialogAct)>,
    pub sel: usize,
    pub opened: Instant,
    pub bar: f32,
}

impl App {
    pub fn no_emulator_dialog(&mut self, s: System, game: usize) {
        let mut options = Vec::new();
        let body = match s.core() {
            Some(c) => {
                options.push((format!("Download {} (one time)", c.name), DialogAct::DownloadCore(s, game)));
                format!(
                    "Choose your {} emulator in Settings, or download the built-in {} and play right away.",
                    s.emu_label(),
                    c.name
                )
            }
            None => format!("Choose your {} emulator ({}) in Settings to play this game.", s.emu_label(), s.emulator_hint()),
        };
        options.push(("Choose Emulator".into(), DialogAct::OpenSettings(s)));
        options.push(("Cancel".into(), DialogAct::Close));
        self.dialog = Some(Dialog { title: format!("No {} emulator", s.short()), body, options, sel: 0, opened: Instant::now(), bar: 0.0 });
        self.sounds.play("select");
    }

    /// "Where is the emulator?" with plain choices.
    pub fn emulator_dialog(&mut self, s: System) {
        let mut options = vec![
            (format!("Choose the program ({})", s.emulator_hint()), DialogAct::Row(RowId::EmulatorExe(s))),
            ("Choose the folder it's in".to_string(), DialogAct::Row(RowId::EmulatorFolder(s))),
        ];
        if !self.cfg.sys(s).emulator.is_empty() {
            options.push(("Forget it (detect automatically)".into(), DialogAct::Row(RowId::EmulatorClear(s))));
        }
        options.push(("Cancel".into(), DialogAct::Close));
        let body = format!(
            "Pick whichever is easier. The program is the file you double-click to open {} (for example {}). If you choose a folder instead, the launcher finds that program inside it for you.",
            s.emu_label(),
            s.emulator_hint()
        );
        self.dialog = Some(Dialog { title: format!("Where is {}?", s.emu_label()), body, options, sel: 0, opened: Instant::now(), bar: 0.0 });
    }

    pub fn dialog_input(&mut self, p: Pad, frame: &eframe::Frame) {
        let Some(d) = self.dialog.as_mut() else { return };
        match p {
            Pad::Up if d.sel > 0 => {
                d.sel -= 1;
                self.sounds.play("move");
            }
            Pad::Down if d.sel + 1 < d.options.len() => {
                d.sel += 1;
                self.sounds.play("move");
            }
            Pad::B => {
                self.dialog = None;
                self.sounds.play("back");
            }
            Pad::A => {
                let act = d.options[d.sel].1;
                self.dialog = None;
                self.sounds.play("select");
                match act {
                    DialogAct::DownloadCore(s, g) => {
                        // The user chose the core for this console: remember it, download, then play.
                        self.cfg.sys_mut(s).backend = "core".into();
                        self.cfg.save();
                        self.install_core(s, Some(g), true);
                    }
                    DialogAct::OpenSettings(s) => {
                        let mut m = Menu::new(Kind::Settings);
                        m.cat = CAT_CONSOLES;
                        m.in_list = true;
                        m.console = Some(s);
                        self.menu = Some(m);
                    }
                    DialogAct::Row(id) => {
                        let ctx = self.ctx().cloned();
                        if let Some(ctx) = ctx {
                            self.activate_row(id, frame, &ctx);
                        }
                    }
                    DialogAct::Close => {}
                }
            }
            _ => {}
        }
    }
}

pub fn draw_dialog(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(d) = app.dialog.as_mut() else { return };
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let (w, h) = (screen.width(), screen.height());
    let u = (h / 720.0).min(w / 1280.0);
    let t = d.opened.elapsed().as_secs_f32();
    let a = (t / 0.2).min(1.0);
    let e = 1.0 - (1.0 - a).powi(3);
    p.rect_filled(screen, 0.0, Color32::from_black_alpha((150.0 * a) as u8));
    let pw = 560.0 * u;
    let body_g = p.layout(d.body.clone(), FontId::proportional(19.0 * u), Color32::WHITE, pw - 56.0 * u);
    let row_h = 44.0 * u;
    let ph = 70.0 * u + body_g.size().y + 22.0 * u + d.options.len() as f32 * row_h + 18.0 * u;
    let panel = Rect::from_center_size(pos2(w * 0.5, h * 0.5 + (1.0 - e) * 30.0 * u), vec2(pw, ph));
    let rad = CornerRadius::same((6.0 * u) as u8);
    p.rect_filled(panel.translate(vec2(0.0, 4.0 * u)), rad, Color32::from_black_alpha((60.0 * a) as u8));
    p.rect_filled(panel, rad, Color32::from_rgba_unmultiplied(66, 82, 100, (245.0 * a) as u8));
    p.rect_stroke(panel, rad, Stroke::new(1.0, Color32::from_white_alpha((30.0 * a) as u8)), StrokeKind::Inside);
    text_shadow(&p, panel.min + vec2(28.0 * u, 38.0 * u), Align2::LEFT_CENTER, &d.title, 25.0 * u, Color32::WHITE.gamma_multiply(a));
    p.galley(panel.min + vec2(28.0 * u, 64.0 * u), body_g, Color32::WHITE);
    let list_top = panel.top() + 70.0 * u + p.layout(d.body.clone(), FontId::proportional(19.0 * u), Color32::WHITE, pw - 56.0 * u).size().y + 18.0 * u;
    d.bar += (d.sel as f32 - d.bar) * 0.35;
    let f = Frame360 {
        u,
        panel,
        list: Rect::from_min_max(pos2(panel.left() + 18.0 * u, list_top), pos2(panel.right() - 18.0 * u, panel.bottom())),
        right: panel,
        row_h,
    };
    sel_bar(&p, &f, d.bar, a);
    BAR_EXTERNAL.with(|b| b.set(true));
    for (i, (label, _)) in d.options.iter().enumerate() {
        list_row(&p, &f, i, label, None, i == d.sel, true, a, false);
    }
    BAR_EXTERNAL.with(|b| b.set(false));
    hint_bar(&p, &[("A", "Select", BTN_A), ("B", "Back", BTN_B)], 100.0 * u, 650.0 * u, u, a);
}
