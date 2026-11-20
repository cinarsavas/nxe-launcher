//! Per-console knowledge: emulator exe names, launch arguments, and where each
//! emulator keeps its own game list so the launcher can import it.

use crate::config::base_dir;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum System {
    Xbox360,
    Ps3,
    Ps2,
    Switch,
    Gc,
    N64,
    Ds,
    N3ds,
    Psp,
    Ps1,
    Xbox,
    /// Native PC ports and recompilations (Zelda 64: Recompiled, Unleashed Recompiled, …).
    Pc,
}

pub const ALL: [System; 12] = [
    System::Xbox360,
    System::Xbox,
    System::Ps3,
    System::Ps2,
    System::Ps1,
    System::Psp,
    System::Switch,
    System::Gc,
    System::N64,
    System::Ds,
    System::N3ds,
    System::Pc,
];

/// A libretro core that can run this console when no emulator is installed.
pub struct CoreInfo {
    /// e.g. "dolphin_libretro"
    pub file: &'static str,
    pub name: &'static str,
    /// Extra files RetroArch's "core system files" updater would fetch.
    pub system_zip: Option<&'static str>,
    pub note: &'static str,
}

impl System {
    pub fn channel(self) -> &'static str {
        match self {
            System::Xbox360 => "Xbox 360",
            System::Ps3 => "PlayStation 3",
            System::Ps2 => "PlayStation 2",
            System::Switch => "Switch",
            System::Gc => "GameCube",
            System::N64 => "Nintendo 64",
            System::Ds => "Nintendo DS",
            System::N3ds => "Nintendo 3DS",
            System::Psp => "PSP",
            System::Ps1 => "PlayStation",
            System::Xbox => "Xbox",
            System::Pc => "PC Ports",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            System::Xbox360 => "Xbox 360",
            System::Ps3 => "PS3",
            System::Ps2 => "PS2",
            System::Switch => "Switch",
            System::Gc => "GameCube",
            System::N64 => "N64",
            System::Ds => "DS",
            System::N3ds => "3DS",
            System::Psp => "PSP",
            System::Ps1 => "PS1",
            System::Xbox => "Xbox",
            System::Pc => "PC",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            System::Xbox360 => "xbox360",
            System::Ps3 => "ps3",
            System::Ps2 => "ps2",
            System::Switch => "switch",
            System::Gc => "gc",
            System::N64 => "n64",
            System::Ds => "ds",
            System::N3ds => "3ds",
            System::Psp => "psp",
            System::Ps1 => "ps1",
            System::Xbox => "xbox",
            System::Pc => "pc",
        }
    }
    pub fn emu_label(self) -> &'static str {
        match self {
            System::Xbox360 => "Xenia",
            System::Ps3 => "RPCS3",
            System::Ps2 => "PCSX2",
            System::Switch => "Eden / Citron / Ryujinx",
            System::Gc => "Dolphin",
            System::N64 => "RMG / simple64 / Project64",
            System::Ds => "melonDS",
            System::N3ds => "Azahar",
            System::Psp => "PPSSPP",
            System::Ps1 => "DuckStation",
            System::Xbox => "xemu",
            System::Pc => "GitHub Launcher",
        }
    }
    pub fn core(self) -> Option<CoreInfo> {
        match self {
            System::Gc => Some(CoreInfo { file: "dolphin_libretro", name: "Dolphin core", system_zip: Some("Dolphin.zip"), note: "" }),
            System::N64 => Some(CoreInfo { file: "mupen64plus_next_libretro", name: "Mupen64Plus-Next core", system_zip: None, note: "" }),
            System::Ds => Some(CoreInfo { file: "melondsds_libretro", name: "melonDS DS core", system_zip: None, note: "" }),
            System::N3ds => Some(CoreInfo { file: "azahar_libretro", name: "Azahar core", system_zip: None, note: "Games must be decrypted (.cci / .3ds / .cxi)." }),
            System::Xbox360 => Some(CoreInfo { file: "xenia_edge_libretro", name: "Xenia core (experimental)", system_zip: None, note: "Experimental. Many games run better in standalone Xenia." }),
            System::Ps3 => Some(CoreInfo {
                file: "rpcs3_libretro",
                name: "RPCS3 core (experimental)",
                system_zip: None,
                note: "Experimental. Needs the PS3 firmware installed, the same as standalone RPCS3.",
            }),
            System::Psp => Some(CoreInfo { file: "ppsspp_libretro", name: "PPSSPP core", system_zip: Some("PPSSPP.zip"), note: "" }),
            System::Ps1 => Some(CoreInfo {
                file: "swanstation_libretro",
                name: "SwanStation core (DuckStation-based)",
                system_zip: None,
                note: "Needs a PS1 BIOS file (e.g. scph5501.bin) in runtime\\system.",
            }),
            System::Xbox => Some(CoreInfo {
                file: "xemu_libretro",
                name: "xemu core (experimental)",
                system_zip: None,
                note: "Needs the Xbox MCPX boot ROM, BIOS and HDD image in runtime\\system, the same files standalone xemu uses.",
            }),
            System::Ps2 => Some(CoreInfo {
                file: "pcsx2_libretro",
                name: "LRPS2 core",
                system_zip: Some("LRPS2.zip"),
                note: "Needs your PS2 BIOS in runtime\\system\\pcsx2\\bios.",
            }),
            _ => None,
        }
    }
    pub fn cover_repo(self) -> Option<&'static str> {
        Some(match self {
            System::Xbox360 => "Microsoft_-_Xbox_360",
            System::Ps3 => "Sony_-_PlayStation_3",
            System::Ps2 => "Sony_-_PlayStation_2",
            System::Gc => "Nintendo_-_GameCube",
            System::N64 => "Nintendo_-_Nintendo_64",
            System::Ds => "Nintendo_-_Nintendo_DS",
            System::N3ds => "Nintendo_-_Nintendo_3DS",
            System::Psp => "Sony_-_PlayStation_Portable",
            System::Ps1 => "Sony_-_PlayStation",
            System::Xbox => "Microsoft_-_Xbox",
            System::Switch | System::Pc => return None,
        })
    }
    /// RetroAchievements console ID, for consoles RA supports.
    pub fn ra_console(self) -> Option<u64> {
        match self {
            System::Ps2 => Some(21),
            System::Gc => Some(16),
            System::N64 => Some(2),
            System::Ds => Some(18),
            System::Psp => Some(41),
            System::Ps1 => Some(12),
            _ => None,
        }
    }
    pub fn emulator_hint(self) -> &'static str {
        match self {
            System::Xbox360 => "xenia_canary.exe",
            System::Ps3 => "rpcs3.exe",
            System::Ps2 => "pcsx2-qt.exe",
            System::Switch => "eden.exe / citron.exe / Ryujinx.exe",
            System::Gc => "Dolphin.exe",
            System::N64 => "RMG.exe / simple64-gui.exe / Project64.exe",
            System::Ds => "melonDS.exe",
            System::N3ds => "azahar.exe",
            System::Psp => "PPSSPPWindows64.exe",
            System::Ps1 => "duckstation-qt-x64-ReleaseLTCG.exe",
            System::Xbox => "xemu.exe",
            System::Pc => "GithubLauncher.exe",
        }
    }
    pub fn folder_hint(self) -> &'static str {
        match self {
            System::Xbox360 => ".iso, .zar, default.xex or GOD packages",
            System::Ps3 => "game folders (PS3_GAME / EBOOT.BIN) or .iso",
            System::Ps2 => ".iso, .chd, .cso, .cue",
            System::Switch => ".nsp, .xci, .nsz, .xcz",
            System::Gc => ".iso, .rvz, .gcz, .gcm, .ciso, .wbfs",
            System::N64 => ".z64, .n64, .v64, .zip",
            System::Ds => ".nds",
            System::N3ds => ".3ds, .cci, .cxi, .3dsx",
            System::Psp => ".iso, .cso, .chd, .pbp",
            System::Ps1 => ".cue, .chd, .pbp, .m3u, .iso",
            System::Xbox => ".iso (XISO format)",
            System::Pc => "a port's folder, or a folder with several ports in it",
        }
    }
    fn exe_names(self) -> &'static [&'static str] {
        match self {
            System::Xbox360 => &["xenia_canary.exe", "xenia.exe", "xenia_edge.exe", "xenia_canary", "xenia"],
            System::Ps3 => &["rpcs3.exe", "rpcs3"],
            System::Ps2 => &["pcsx2-qt.exe", "pcsx2-qtx64-avx2.exe", "pcsx2-qtx64.exe", "pcsx2.exe", "pcsx2-qt", "pcsx2"],
            System::Switch => &[
                "eden.exe", "citron.exe", "Ryujinx.exe", "ryujinx.exe", "sudachi.exe", "yuzu.exe",
                "eden", "citron", "Ryujinx",
            ],
            System::Gc => &["Dolphin.exe", "dolphin-emu", "DolphinQt.exe"],
            System::N64 => &["RMG.exe", "simple64-gui.exe", "Project64.exe", "RMG"],
            System::Ds => &["melonDS.exe", "melonDS"],
            System::N3ds => &["azahar.exe", "lime3ds.exe", "lime3ds-gui.exe", "citra-qt.exe", "azahar"],
            System::Psp => &["PPSSPPWindows64.exe", "PPSSPPWindows.exe", "PPSSPPSDL", "ppsspp"],
            System::Ps1 => &[
                "duckstation-qt-x64-ReleaseLTCG.exe",
                "duckstation-qt-ARM64-ReleaseLTCG.exe",
                "duckstation-qt.exe",
                "duckstation-qt",
                "duckstation",
            ],
            System::Xbox => &["xemu.exe", "xemu"],
            System::Pc => &["GithubLauncher.exe", "N64RecompLauncher.exe", "GithubLauncher", "N64RecompLauncher"],
        }
    }

    /// Finds this emulator's program inside a folder the user picked (searches 2 levels deep).
    pub fn find_in(self, dir: &Path) -> Option<PathBuf> {
        fn walk(s: System, d: &Path, depth: usize) -> Option<PathBuf> {
            for n in s.exe_names() {
                let p = d.join(n);
                if p.is_file() {
                    return Some(p);
                }
            }
            if depth == 0 {
                return None;
            }
            let mut subs: Vec<PathBuf> = std::fs::read_dir(d).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            subs.sort();
            subs.iter().find_map(|sd| walk(s, sd, depth - 1))
        }
        walk(self, dir, 2)
    }

    /// Looks next to the launcher and one folder down (e.g. .\rpcs3\rpcs3.exe).
    pub fn detect(self) -> Option<PathBuf> {
        let base = base_dir();
        let mut dirs = vec![base.clone()];
        if let Ok(rd) = std::fs::read_dir(&base) {
            let mut subs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            subs.sort();
            dirs.extend(subs);
        }
        if self == System::Ps2 {
            if let Ok(pf) = std::env::var("ProgramFiles") {
                dirs.push(PathBuf::from(pf).join("PCSX2"));
            }
        }
        for d in dirs {
            for n in self.exe_names() {
                let p = d.join(n);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Arguments before the game path when the user hasn't set any.
    pub fn default_args(self, exe: &Path) -> Vec<String> {
        let name = exe.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let v: &[&str] = match self {
            System::Xbox360 => &["--fullscreen=true"],
            // -batch: PCSX2 quits when the game is shut down; -nogui: no main window.
            System::Ps2 => &["-fullscreen", "-nogui", "-batch", "--"],
            // --no-gui: RPCS3 exits when the game window closes.
            System::Ps3 => &["--no-gui", "--fullscreen"],
            System::Switch if name.contains("ryujinx") => &["--fullscreen"],
            // yuzu family (Eden, Citron, Sudachi): -f fullscreen, -g <game>
            System::Switch => &["-f", "-g"],
            // -b: quit when the game stops; -e: run this file.
            System::Gc => &["-b", "-C", "Dolphin.Display.Fullscreen=True", "-e"],
            System::N64 if name.starts_with("rmg") => &["--fullscreen", "--quit-after-emulation"],
            System::N64 if name.starts_with("simple64") => &["--nogui"],
            System::N64 => &[],
            System::Ds => &["-f"],
            System::N3ds => &["-f"],
            // --escape-exit: Esc / pause closes PPSSPP straight back to the dashboard.
            System::Psp => &["--fullscreen", "--escape-exit"],
            System::Ps1 => &["-fullscreen", "-batch", "--"],
            System::Xbox => &["-full-screen", "-dvd_path"],
            System::Pc => &[],
        };
        v.iter().map(|s| s.to_string()).collect()
    }

    /// Folders the emulator itself is already set up with.
    pub fn imported_dirs(self, exe: &Path) -> Vec<PathBuf> {
        if self == System::Pc {
            return github_launcher_apps(exe).into_iter().map(|(_, p)| p).collect();
        }
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).map(PathBuf::from).ok();
        let appdata = std::env::var("APPDATA").map(PathBuf::from).ok();
        let mut out = Vec::new();
        match self {
            System::Ps2 => {
                // PCSX2.ini: [GameList] Paths = ... / RecursivePaths = ...
                let mut inis = vec![dir.join("inis").join("PCSX2.ini")];
                if let Some(h) = &home {
                    inis.push(h.join("Documents").join("PCSX2").join("inis").join("PCSX2.ini"));
                }
                for ini in inis {
                    if let Ok(s) = std::fs::read_to_string(&ini) {
                        let mut in_list = false;
                        for line in s.lines() {
                            let l = line.trim();
                            if l.starts_with('[') {
                                in_list = l.eq_ignore_ascii_case("[GameList]");
                                continue;
                            }
                            if in_list {
                                if let Some((k, v)) = l.split_once('=') {
                                    let k = k.trim();
                                    if k == "Paths" || k == "RecursivePaths" {
                                        out.push(PathBuf::from(v.trim()));
                                    }
                                }
                            }
                        }
                        break;
                    }
                }
            }
            System::Ps3 => {
                // Installed games live in dev_hdd0; disc games are listed in games.yml.
                out.push(dir.join("dev_hdd0").join("game"));
                for yml in [dir.join("config").join("games.yml"), dir.join("games.yml")] {
                    if let Ok(s) = std::fs::read_to_string(&yml) {
                        for line in s.lines() {
                            if let Some((_, v)) = line.split_once(": ") {
                                let v = v.trim().trim_matches('"').trim_matches('\'');
                                if !v.is_empty() {
                                    out.push(PathBuf::from(v));
                                }
                            }
                        }
                    }
                }
            }
            System::Switch => {
                let name = exe.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                if name.contains("ryujinx") {
                    // Config.json: "game_dirs": [ "D:\\Switch", ... ]
                    let mut cands = vec![dir.join("portable").join("Config.json")];
                    if let Some(a) = &appdata {
                        cands.push(a.join("Ryujinx").join("Config.json"));
                    }
                    for c in cands {
                        if let Ok(s) = std::fs::read_to_string(&c) {
                            out.extend(json_string_array(&s, "game_dirs").into_iter().map(PathBuf::from));
                            break;
                        }
                    }
                } else {
                    // qt-config.ini: Paths\gamedirs\N\path=...
                    let flavor = ["eden", "citron", "sudachi", "yuzu"]
                        .into_iter()
                        .find(|f| name.contains(f))
                        .unwrap_or("eden");
                    let mut cands = vec![dir.join("user").join("config").join("qt-config.ini")];
                    if let Some(a) = &appdata {
                        cands.push(a.join(flavor).join("config").join("qt-config.ini"));
                    }
                    for c in cands {
                        if let Ok(s) = std::fs::read_to_string(&c) {
                            for line in s.lines() {
                                let l = line.trim();
                                if l.starts_with("Paths\\gamedirs\\") && l.contains("\\path=") {
                                    if let Some((_, v)) = l.split_once('=') {
                                        let v = v.trim();
                                        if !v.is_empty() && !v.starts_with("SDMC") && !v.starts_with("UserNAND") && !v.starts_with("SysNAND") {
                                            out.push(PathBuf::from(v));
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
            System::Gc => {
                // Dolphin.ini: [General] ISOPath0 = ...
                let mut inis = vec![dir.join("User").join("Config").join("Dolphin.ini")];
                if let Some(a) = &appdata {
                    inis.push(a.join("Dolphin Emulator").join("Config").join("Dolphin.ini"));
                }
                if let Some(h) = &home {
                    inis.push(h.join("Documents").join("Dolphin Emulator").join("Config").join("Dolphin.ini"));
                }
                for ini in inis {
                    if let Ok(s) = std::fs::read_to_string(&ini) {
                        for l in s.lines() {
                            if let Some((k, v)) = l.split_once('=') {
                                let k = k.trim();
                                if k.starts_with("ISOPath") && k != "ISOPaths" {
                                    out.push(PathBuf::from(v.trim()));
                                }
                            }
                        }
                        break;
                    }
                }
            }
            System::N3ds => {
                let mut cands = vec![dir.join("user").join("config").join("qt-config.ini")];
                if let Some(a) = &appdata {
                    for f in ["Azahar", "Lime3DS", "Citra"] {
                        cands.push(a.join(f).join("config").join("qt-config.ini"));
                    }
                }
                for c in cands {
                    if let Ok(s) = std::fs::read_to_string(&c) {
                        out.extend(qt_gamedirs(&s));
                        break;
                    }
                }
            }
            System::Xbox360 | System::N64 | System::Ds | System::Psp | System::Ps1 | System::Xbox | System::Pc => {}
        }
        out.retain(|p| p.is_dir());
        out
    }

    /// Extra places to look for cover art for this system.
    pub fn emulator_cover_dirs(self, exe: Option<&Path>) -> Vec<PathBuf> {
        let mut v = Vec::new();
        if self == System::Ps2 {
            if let Some(d) = exe.and_then(|e| e.parent()) {
                v.push(d.join("covers"));
            }
            if let Ok(h) = std::env::var("USERPROFILE") {
                v.push(PathBuf::from(h).join("Documents").join("PCSX2").join("covers"));
            }
        }
        v
    }
}

/// `Paths\gamedirs\N\path=...` entries from a Citra/yuzu-family qt-config.ini.
fn qt_gamedirs(s: &str) -> Vec<PathBuf> {
    s.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("Paths\\gamedirs\\") && l.contains("\\path="))
        .filter_map(|l| l.split_once('=').map(|(_, v)| v.trim().to_string()))
        .filter(|v| !v.is_empty() && !matches!(v.as_str(), "SDMC" | "UserNAND" | "SysNAND" | "INSTALLED" | "SYSTEM"))
        .map(PathBuf::from)
        .collect()
}

/// Minimal extraction of `"key": ["a", "b"]` from JSON (enough for Ryujinx's config).
fn json_string_array(s: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let Some(i) = s.find(&needle) else { return vec![] };
    let rest = &s[i + needle.len()..];
    let Some(open) = rest.find('[') else { return vec![] };
    let Some(close) = rest[open..].find(']') else { return vec![] };
    let body = &rest[open + 1..open + close];
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut esc = false;
    for c in body.chars() {
        if in_str {
            if esc {
                cur.push(c);
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
                out.push(std::mem::take(&mut cur));
            } else {
                cur.push(c);
            }
        } else if c == '"' {
            in_str = true;
        }
    }
    out
}

/// Apps installed by GitHub Launcher (SirDiabo, formerly N64RecompLauncher): `apps.json`
/// (or the older `games.json`) next to its exe lists them; each one lives in `installPath`,
/// or `<AppsPath from settings.json, default "Apps">\<folderName>`.
pub fn github_launcher_apps(exe: &Path) -> Vec<(String, PathBuf)> {
    let base = if exe.is_dir() { exe.to_path_buf() } else { exe.parent().map(Path::to_path_buf).unwrap_or_default() };
    let read = |n: &str| std::fs::read_to_string(base.join(n)).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}')).ok());
    let apps_dir = read("settings.json")
        .and_then(|v| v["AppsPath"].as_str().filter(|s| !s.trim().is_empty()).map(PathBuf::from))
        .unwrap_or_else(|| base.join("Apps"));
    let mut out = Vec::new();
    for file in ["apps.json", "games.json"] {
        let Some(v) = read(file) else { continue };
        let mut entries: Vec<&serde_json::Value> = Vec::new();
        for sec in ["apps", "standard", "experimental", "custom"] {
            if let Some(a) = v[sec].as_array() {
                entries.extend(a.iter());
            }
        }
        if let Some(a) = v.as_array() {
            entries.extend(a.iter());
        }
        for e in entries {
            let name = e["name"].as_str().unwrap_or_default().to_string();
            let path = match e["installPath"].as_str().filter(|s| !s.trim().is_empty()) {
                Some(p) => PathBuf::from(p),
                None => match e["folderName"].as_str().filter(|s| !s.trim().is_empty()) {
                    Some(f) => apps_dir.join(f),
                    None => continue,
                },
            };
            // Only apps that are actually installed.
            if path.is_dir() && !out.iter().any(|(_, p): &(String, PathBuf)| p == &path) {
                out.push((name, path));
            }
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}
