//! Basic emulator settings (resolution, aspect ratio, VSync, fullscreen)
//! edited straight in each emulator's own config file, so the change is the
//! same as making it inside the emulator.

use crate::systems::System;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Opt {
    pub label: &'static str,
    pub desc: &'static str,
    pub file: PathBuf,
    /// (section, key) pairs that all get the same value.
    pub keys: Vec<(&'static str, &'static str)>,
    /// (shown text, value written to the file)
    pub choices: Vec<(&'static str, &'static str)>,
    pub toml: bool,
}

fn first_existing(c: Vec<PathBuf>) -> Option<PathBuf> {
    c.into_iter().find(|p| p.is_file())
}

fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var(var).ok().map(PathBuf::from)
}

/// Where the emulator keeps its settings, if it has been run at least once.
pub fn config_file(s: System, exe: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
    let dir = exe.parent()?.to_path_buf();
    let docs = env_path("USERPROFILE").map(|h| h.join("Documents"));
    let appdata = env_path("APPDATA");
    let local = env_path("LOCALAPPDATA");
    match s {
        System::Ps2 => {
            let mut c = vec![dir.join("inis").join("PCSX2.ini")];
            if let Some(d) = &docs {
                c.push(d.join("PCSX2").join("inis").join("PCSX2.ini"));
            }
            first_existing(c).map(|p| (p, None))
        }
        System::Gc => {
            let mut dirs = vec![dir.join("User").join("Config")];
            if let Some(a) = &appdata {
                dirs.push(a.join("Dolphin Emulator").join("Config"));
            }
            if let Some(d) = &docs {
                dirs.push(d.join("Dolphin Emulator").join("Config"));
            }
            dirs.into_iter().find(|d| d.join("Dolphin.ini").is_file()).map(|d| (d.join("GFX.ini"), Some(d.join("Dolphin.ini"))))
        }
        System::Ps1 => {
            let mut c = vec![dir.join("settings.ini")];
            if let Some(l) = &local {
                c.push(l.join("DuckStation").join("settings.ini"));
            }
            if let Some(d) = &docs {
                c.push(d.join("DuckStation").join("settings.ini"));
            }
            first_existing(c).map(|p| (p, None))
        }
        System::Psp => {
            let mut c = vec![dir.join("memstick").join("PSP").join("SYSTEM").join("ppsspp.ini")];
            if let Some(d) = &docs {
                c.push(d.join("PPSSPP").join("PSP").join("SYSTEM").join("ppsspp.ini"));
            }
            first_existing(c).map(|p| (p, None))
        }
        System::Xbox360 => first_existing(vec![
            dir.join("xenia-canary.config.toml"),
            dir.join("xenia-edge.config.toml"),
            dir.join("xenia.config.toml"),
        ])
        .map(|p| (p, None)),
        System::Xbox => {
            let mut c = vec![dir.join("xemu.toml")];
            if let Some(a) = &appdata {
                c.push(a.join("xemu").join("xemu").join("xemu.toml"));
            }
            first_existing(c).map(|p| (p, None))
        }
        _ => None,
    }
}

const ON_OFF: &[(&str, &str)] = &[("On", "true"), ("Off", "false")];
const ON_OFF_CAP: &[(&str, &str)] = &[("On", "True"), ("Off", "False")];

/// The basic options this launcher can change for an emulator (empty if none).
pub fn options(s: System, exe: &Path) -> Vec<Opt> {
    let Some((main, extra)) = config_file(s, exe) else { return vec![] };
    let o = |label, desc, file: &PathBuf, keys: Vec<(&'static str, &'static str)>, choices: &[(&'static str, &'static str)], toml| Opt {
        label,
        desc,
        file: file.clone(),
        keys,
        choices: choices.to_vec(),
        toml,
    };
    match s {
        System::Ps2 => vec![
            o(
                "Resolution",
                "How sharp 3D graphics look. Higher needs a stronger graphics card.",
                &main,
                vec![("EmuCore/GS", "upscale_multiplier")],
                &[("Native (PS2)", "1"), ("2x (720p)", "2"), ("3x (1080p)", "3"), ("4x (1440p)", "4"), ("6x (4K)", "6")],
                false,
            ),
            o(
                "Aspect Ratio",
                "Picture shape. Auto uses what the game asks for.",
                &main,
                vec![("EmuCore/GS", "AspectRatio")],
                &[("Auto", "Auto 4:3/3:2"), ("4:3", "4:3"), ("16:9", "16:9"), ("Stretch", "Stretch")],
                false,
            ),
            o("VSync", "Stops screen tearing. Turn off if a game feels laggy.", &main, vec![("EmuCore/GS", "VsyncEnable")], ON_OFF, false),
            o("Start Fullscreen", "Open games fullscreen.", &main, vec![("UI", "StartFullscreen")], ON_OFF, false),
        ],
        System::Gc => {
            let dolphin = extra.unwrap_or_else(|| main.with_file_name("Dolphin.ini"));
            vec![
                o(
                    "Resolution",
                    "How sharp 3D graphics look. Higher needs a stronger graphics card.",
                    &main,
                    vec![("Settings", "InternalResolution")],
                    &[("Native (480p)", "1"), ("2x (720p)", "2"), ("3x (1080p)", "3"), ("4x (1440p)", "4"), ("6x (4K)", "6")],
                    false,
                ),
                o(
                    "Aspect Ratio",
                    "Picture shape. Auto uses what the game asks for.",
                    &main,
                    vec![("Settings", "AspectRatio")],
                    &[("Auto", "0"), ("16:9", "1"), ("4:3", "2"), ("Stretch", "3")],
                    false,
                ),
                o("VSync", "Stops screen tearing. Turn off if a game feels laggy.", &main, vec![("Hardware", "VSync")], ON_OFF_CAP, false),
                o("Start Fullscreen", "Open games fullscreen.", &dolphin, vec![("Display", "Fullscreen")], ON_OFF_CAP, false),
            ]
        }
        System::Ps1 => vec![
            o(
                "Resolution",
                "How sharp 3D graphics look. PS1 games are very low resolution, so even 4x looks much cleaner.",
                &main,
                vec![("GPU", "ResolutionScale")],
                &[("Native (240p)", "1"), ("2x (480p)", "2"), ("3x (720p)", "3"), ("4x (960p)", "4"), ("6x (1440p)", "6"), ("8x (4K)", "8")],
                false,
            ),
            o(
                "Aspect Ratio",
                "Picture shape. Auto uses what the game asks for.",
                &main,
                vec![("Display", "AspectRatio")],
                &[("Auto", "Auto (Game Native)"), ("4:3", "4:3"), ("16:9", "16:9")],
                false,
            ),
            o("VSync", "Stops screen tearing. Turn off if a game feels laggy.", &main, vec![("Display", "VSync")], ON_OFF, false),
            o("Start Fullscreen", "Open games fullscreen.", &main, vec![("Main", "StartFullscreen")], ON_OFF, false),
        ],
        System::Psp => vec![
            o(
                "Resolution",
                "How sharp 3D graphics look. 4x is about 1080p.",
                &main,
                vec![("Graphics", "InternalResolution")],
                &[("Native (272p)", "1"), ("2x", "2"), ("3x (720p)", "3"), ("4x (1080p)", "4"), ("6x (1440p)", "6"), ("8x (4K)", "8")],
                false,
            ),
            o("VSync", "Stops screen tearing. Turn off if a game feels laggy.", &main, vec![("Graphics", "VSync")], ON_OFF_CAP, false),
            o("Start Fullscreen", "Open games fullscreen.", &main, vec![("Graphics", "FullScreen")], ON_OFF_CAP, false),
        ],
        System::Xbox360 => vec![
            o(
                "Resolution",
                "Internal rendering scale. 2x needs a strong graphics card.",
                &main,
                vec![("GPU", "draw_resolution_scale_x"), ("GPU", "draw_resolution_scale_y")],
                &[("Native (720p)", "1"), ("2x (1440p)", "2"), ("3x (4K)", "3")],
                true,
            ),
            o("VSync", "Stops screen tearing. Turn off if a game feels laggy.", &main, vec![("GPU", "vsync")], ON_OFF, true),
            o("Start Fullscreen", "Open games fullscreen.", &main, vec![("Display", "fullscreen")], ON_OFF, true),
        ],
        System::Xbox => vec![
            o(
                "Resolution",
                "Surface scale. 2x is about 960p.",
                &main,
                vec![("display.quality", "surface_scale")],
                &[("Native (480p)", "1"), ("2x", "2"), ("3x (1440p)", "3"), ("4x", "4")],
                true,
            ),
            o("VSync", "Stops screen tearing.", &main, vec![("display.window", "vsync")], ON_OFF, true),
            o("Start Fullscreen", "Open games fullscreen.", &main, vec![("display.window", "fullscreen_on_startup")], ON_OFF, true),
        ],
        _ => vec![],
    }
}

/// Current value of an option, shown as its choice text.
pub fn current(o: &Opt) -> String {
    let (sec, key) = o.keys[0];
    let raw = read(&o.file, sec, key);
    match raw {
        Some(v) => {
            let v = v.trim_matches('"');
            o.choices
                .iter()
                .find(|(_, val)| val.eq_ignore_ascii_case(v) || v.parse::<f32>().ok().zip(val.parse::<f32>().ok()).map(|(a, b)| a == b).unwrap_or(false))
                .map(|(t, _)| t.to_string())
                .unwrap_or_else(|| v.to_string())
        }
        None => "Emulator default".into(),
    }
}

/// Move to the next choice and write it.
pub fn cycle(o: &Opt) -> std::io::Result<String> {
    let cur = current(o);
    let i = o.choices.iter().position(|(t, _)| *t == cur).map(|i| (i + 1) % o.choices.len()).unwrap_or(0);
    let (text, val) = o.choices[i];
    for (sec, key) in &o.keys {
        let v = if o.toml && val.parse::<f64>().is_err() && val != "true" && val != "false" { format!("\"{val}\"") } else { val.to_string() };
        write(&o.file, sec, key, &v)?;
    }
    Ok(text.to_string())
}

fn read(file: &Path, section: &str, key: &str) -> Option<String> {
    let s = std::fs::read_to_string(file).ok()?;
    let mut in_sec = false;
    for l in s.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_sec = t.trim_start_matches('[').trim_end_matches(']').trim().eq_ignore_ascii_case(section);
            continue;
        }
        if in_sec {
            if let Some((k, v)) = t.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    // Drop trailing TOML comments.
                    let v = v.split(" #").next().unwrap_or(v);
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// Set `key = value` inside `[section]`, keeping everything else in the file.
fn write(file: &Path, section: &str, key: &str, value: &str) -> std::io::Result<()> {
    let s = std::fs::read_to_string(file).unwrap_or_default();
    let nl = if s.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = s.lines().map(String::from).collect();
    let mut sec_start = None;
    let mut sec_end = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        if t.starts_with('[') {
            let name = t.trim_start_matches('[').trim_end_matches(']').trim();
            if sec_start.is_some() {
                sec_end = i;
                break;
            }
            if name.eq_ignore_ascii_case(section) {
                sec_start = Some(i);
            }
        }
    }
    let line = format!("{key} = {value}");
    match sec_start {
        Some(st) => {
            let found = (st + 1..sec_end).find(|&i| lines[i].split_once('=').map(|(k, _)| k.trim().eq_ignore_ascii_case(key)).unwrap_or(false));
            match found {
                Some(i) => lines[i] = line,
                None => {
                    // Insert after the last non-empty line of the section.
                    let mut at = sec_end;
                    while at > st + 1 && lines[at - 1].trim().is_empty() {
                        at -= 1;
                    }
                    lines.insert(at, line);
                }
            }
        }
        None => {
            if !lines.last().map(|l| l.trim().is_empty()).unwrap_or(true) {
                lines.push(String::new());
            }
            lines.push(format!("[{section}]"));
            lines.push(line);
        }
    }
    let mut out = lines.join(nl);
    out.push_str(nl);
    // Keep a one-time backup of the original file.
    let bak = file.with_extension("nxe-backup");
    if !bak.exists() {
        let _ = std::fs::write(&bak, &s);
    }
    std::fs::write(file, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ini_roundtrip() {
        let p = std::env::temp_dir().join("nxe_test.ini");
        std::fs::write(&p, "[UI]\r\nTheme = dark\r\n\r\n[EmuCore/GS]\r\nupscale_multiplier = 1.000000\r\nVsyncEnable = false\r\n").unwrap();
        let o = Opt {
            label: "Resolution",
            desc: "",
            file: p.clone(),
            keys: vec![("EmuCore/GS", "upscale_multiplier")],
            choices: vec![("Native", "1"), ("2x", "2")],
            toml: false,
        };
        assert_eq!(current(&o), "Native");
        assert_eq!(cycle(&o).unwrap(), "2x");
        assert_eq!(current(&o), "2x");
        write(&p, "Main", "StartFullscreen", "true").unwrap();
        let s = std::fs::read_to_string(&p).unwrap();
        assert!(s.contains("upscale_multiplier = 2\r\n"));
        assert!(s.contains("Theme = dark"));
        assert!(s.contains("[Main]\r\nStartFullscreen = true"));
        let t = std::env::temp_dir().join("nxe_test.toml");
        std::fs::write(&t, "[display.quality]\nsurface_scale = 1\n").unwrap();
        let o = Opt { label: "", desc: "", file: t.clone(), keys: vec![("display.quality", "surface_scale")], choices: vec![("1", "1"), ("2", "2")], toml: true };
        cycle(&o).unwrap();
        assert!(std::fs::read_to_string(&t).unwrap().contains("surface_scale = 2"));
    }
}
