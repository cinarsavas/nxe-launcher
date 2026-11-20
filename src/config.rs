use crate::systems::System;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONFIG_NAME: &str = "nxe-launcher.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SystemCfg {
    /// Full path to the emulator exe. Empty = auto-detect next to the launcher.
    pub emulator: String,
    /// Arguments placed before the game path. Empty = sensible defaults for that emulator.
    pub args: Vec<String>,
    /// Your own game folders for this system.
    pub game_dirs: Vec<String>,
    /// Also pick up the folders/games the emulator itself already knows about.
    pub import_from_emulator: bool,
    /// "auto" (emulator if installed, otherwise built-in core), "emulator" or "core".
    pub backend: String,
}

impl Default for SystemCfg {
    fn default() -> Self {
        Self { emulator: String::new(), args: vec![], game_dirs: vec![], import_from_emulator: true, backend: "auto".into() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Systems {
    pub xbox360: SystemCfg,
    pub ps3: SystemCfg,
    pub ps2: SystemCfg,
    pub switch: SystemCfg,
    pub gamecube: SystemCfg,
    pub n64: SystemCfg,
    pub ds: SystemCfg,
    #[serde(rename = "3ds")]
    pub n3ds: SystemCfg,
    pub psp: SystemCfg,
    pub ps1: SystemCfg,
    pub xbox: SystemCfg,
    pub pc: SystemCfg,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub gamertag: String,
    pub fullscreen: bool,
    pub minimize_while_playing: bool,
    pub sounds: bool,
    /// First-launch quick setup has been completed.
    pub setup_done: bool,
    /// Download missing cover art in the background.
    pub auto_covers: bool,
    /// Colour theme: green, blue, orange, purple, black.
    pub theme: String,
    /// "builtin:N", "custom" (theme\profile.png) or "ra".
    pub gamerpic: String,
    /// Animation frame-rate cap. 0 = match the display's refresh rate.
    pub frame_rate: u32,
    /// Show "NXE Launcher vX" in the bottom-right corner.
    pub show_version: bool,
    /// Button icons: "auto" (follow the controller), "xbox", "playstation", "nintendo".
    pub button_icons: String,
    /// Nintendo controllers confirm with A (right button), like a Switch.
    pub switch_layout: bool,
    /// Show games you hid.
    pub show_hidden: bool,
    /// Your songs from theme\music on the dashboard.
    pub music_on: bool,
    /// 0-100
    pub music_volume: u32,
    /// Show what you're playing in Discord.
    pub discord_status: bool,
    /// Your Discord application's ID (discord.com/developers).
    pub discord_app_id: String,
    pub check_updates: bool,
    /// RetroAchievements username + session token (the password is never stored).
    pub ra_user: String,
    pub ra_token: String,
    /// Most recently played game paths, newest first (managed by the launcher).
    pub recent: Vec<String>,
    pub systems: Systems,

    // v0.1 fields, migrated into [systems.xbox360] on load.
    #[serde(skip_serializing)]
    xenia_path: Option<String>,
    #[serde(skip_serializing)]
    xenia_args: Option<Vec<String>>,
    #[serde(skip_serializing)]
    game_dirs: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gamertag: "Player".into(),
            fullscreen: true,
            minimize_while_playing: false,
            sounds: true,
            setup_done: false,
            auto_covers: true,
            theme: "green".into(),
            gamerpic: "builtin:0".into(),
            frame_rate: 0,
            show_version: false,
            button_icons: "auto".into(),
            switch_layout: true,
            show_hidden: false,
            music_on: true,
            music_volume: 60,
            discord_status: true,
            discord_app_id: String::new(),
            check_updates: true,
            ra_user: String::new(),
            ra_token: String::new(),
            recent: vec![],
            systems: Systems::default(),
            xenia_path: None,
            xenia_args: None,
            game_dirs: None,
        }
    }
}

/// Folder the launcher lives in (config, theme/ and auto-detected emulators go here).
pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    base_dir().join(CONFIG_NAME)
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => {
                let mut c: Config = toml::from_str(&s).unwrap_or_else(|e| {
                    eprintln!("config parse error, using defaults: {e}");
                    Self::default()
                });
                let mut migrated = false;
                if let Some(p) = c.xenia_path.take() {
                    if c.systems.xbox360.emulator.is_empty() {
                        c.systems.xbox360.emulator = p;
                    }
                    migrated = true;
                }
                if let Some(a) = c.xenia_args.take() {
                    if c.systems.xbox360.args.is_empty() && a != vec!["--fullscreen=true".to_string()] {
                        c.systems.xbox360.args = a;
                    }
                    migrated = true;
                }
                if let Some(d) = c.game_dirs.take() {
                    c.systems.xbox360.game_dirs.extend(d);
                    migrated = true;
                }
                if migrated {
                    c.save();
                }
                c
            }
            Err(_) => {
                let c = Self::default();
                c.save();
                c
            }
        }
    }

    pub fn save(&self) {
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(config_path(), s);
        }
    }

    pub fn sys(&self, s: System) -> &SystemCfg {
        match s {
            System::Xbox360 => &self.systems.xbox360,
            System::Ps3 => &self.systems.ps3,
            System::Ps2 => &self.systems.ps2,
            System::Switch => &self.systems.switch,
            System::Gc => &self.systems.gamecube,
            System::N64 => &self.systems.n64,
            System::Ds => &self.systems.ds,
            System::N3ds => &self.systems.n3ds,
            System::Pc => &self.systems.pc,
            System::Psp => &self.systems.psp,
            System::Ps1 => &self.systems.ps1,
            System::Xbox => &self.systems.xbox,
        }
    }

    pub fn sys_mut(&mut self, s: System) -> &mut SystemCfg {
        match s {
            System::Xbox360 => &mut self.systems.xbox360,
            System::Ps3 => &mut self.systems.ps3,
            System::Ps2 => &mut self.systems.ps2,
            System::Switch => &mut self.systems.switch,
            System::Gc => &mut self.systems.gamecube,
            System::N64 => &mut self.systems.n64,
            System::Ds => &mut self.systems.ds,
            System::N3ds => &mut self.systems.n3ds,
            System::Pc => &mut self.systems.pc,
            System::Psp => &mut self.systems.psp,
            System::Ps1 => &mut self.systems.ps1,
            System::Xbox => &mut self.systems.xbox,
        }
    }

    /// Configured emulator, or one auto-detected next to the launcher.
    pub fn emulator(&self, s: System) -> Option<PathBuf> {
        let c = self.sys(s);
        if !c.emulator.trim().is_empty() {
            let p = PathBuf::from(c.emulator.trim());
            if p.is_file() {
                return Some(p);
            }
        }
        s.detect()
    }

    /// How games for this console will be started. Cores are opt-in: "auto"
    /// only falls back to a core the user has already downloaded.
    pub fn launcher(&self, s: System) -> Launcher {
        if s == System::Pc {
            return Launcher::Native;
        }
        let emu = self.emulator(s);
        let core = s.core().is_some();
        match self.sys(s).backend.as_str() {
            "core" if core => Launcher::Core,
            "emulator" => emu.map(Launcher::Emulator).unwrap_or(Launcher::None),
            _ => match emu {
                Some(e) => Launcher::Emulator(e),
                None if core && crate::runtime::installed(s) => Launcher::Core,
                None => Launcher::None,
            },
        }
    }

    /// Push the controller settings to the input thread.
    pub fn apply_controller(&self) {
        use std::sync::atomic::Ordering;
        let o = match self.button_icons.as_str() {
            "xbox" => 1,
            "playstation" => 2,
            "nintendo" => 3,
            _ => 0,
        };
        crate::input::OVERRIDE.store(o, Ordering::Relaxed);
        crate::input::SWITCH_LAYOUT.store(self.switch_layout, Ordering::Relaxed);
    }

    pub fn push_recent(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(12);
        self.save();
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Launcher {
    Emulator(PathBuf),
    /// Built-in libretro core (downloaded on first use).
    Core,
    /// The game is a program of its own (PC ports).
    Native,
    None,
}
