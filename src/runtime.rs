//! Built-in cores: when a console has no emulator installed, games run on a
//! libretro core inside a private, menu-less RetroArch player that the launcher
//! downloads by itself the first time it's needed (runtime\ next to the exe).
//!
//! The player starts fullscreen straight into the game and quits when the game
//! is closed, so it behaves exactly like a standalone emulator.

use crate::config::base_dir;
use crate::systems::System;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

const BUILDBOT: &str = "https://buildbot.libretro.com";

pub fn root() -> PathBuf {
    base_dir().join("runtime")
}
fn ra_dir() -> PathBuf {
    root().join("RetroArch-Win64")
}
pub fn player_exe() -> PathBuf {
    ra_dir().join("retroarch.exe")
}
pub fn core_path(s: System) -> Option<PathBuf> {
    let c = s.core()?;
    Some(ra_dir().join("cores").join(format!("{}.dll", c.file)))
}
fn marker(zip: &str) -> PathBuf {
    root().join("system").join(format!(".{zip}.installed"))
}

pub fn installed(s: System) -> bool {
    let Some(c) = s.core() else { return false };
    player_exe().is_file()
        && core_path(s).map(|p| p.is_file()).unwrap_or(false)
        && c.system_zip.map(|z| marker(z).is_file()).unwrap_or(true)
}

pub enum Job {
    Install(System),
}

pub enum Msg {
    Progress { system: System, stage: String, frac: f32 },
    Done { system: System, result: Result<(), String> },
}

pub fn spawn(rx: Receiver<Job>, tx: Sender<Msg>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(20)))
            .user_agent(concat!("nxe-launcher/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        while let Ok(Job::Install(s)) = rx.recv() {
            let r = install(&agent, s, &tx, &ctx);
            let _ = tx.send(Msg::Done { system: s, result: r });
            ctx.request_repaint();
        }
    });
}

fn download(agent: &ureq::Agent, url: &str, to: &Path, mut progress: impl FnMut(f32)) -> Result<(), String> {
    let mut resp = agent.get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = resp.body_mut().with_config().limit(u64::MAX).reader();
    let tmp = to.with_extension("part");
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut got = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        if total > 0 {
            progress(got as f32 / total as f32);
        }
    }
    drop(f);
    std::fs::rename(&tmp, to).map_err(|e| e.to_string())
}

fn unzip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    z.extract(dest).map_err(|e| e.to_string())
}

fn install(agent: &ureq::Agent, s: System, tx: &Sender<Msg>, ctx: &eframe::egui::Context) -> Result<(), String> {
    let core = s.core().ok_or("No built-in core for this console")?;
    let root = root();
    std::fs::create_dir_all(root.join("downloads")).map_err(|e| e.to_string())?;
    let send = |stage: &str, frac: f32| {
        let _ = tx.send(Msg::Progress { system: s, stage: stage.to_string(), frac });
        ctx.request_repaint();
    };
    // 1. The player (once for all consoles).
    if !player_exe().is_file() {
        let arc = root.join("downloads").join("RetroArch_update.7z");
        download(agent, &format!("{BUILDBOT}/nightly/windows/x86_64/RetroArch_update.7z"), &arc, |f| send("Downloading the game player", f * 0.7))?;
        send("Unpacking the game player", 0.72);
        sevenz_rust2::decompress_file(&arc, &root).map_err(|e| format!("unpack failed: {e}"))?;
        let _ = std::fs::remove_file(&arc);
    }
    // 2. The core.
    if !core_path(s).map(|p| p.is_file()).unwrap_or(false) {
        let zip = root.join("downloads").join(format!("{}.dll.zip", core.file));
        let base = if player_exe().is_file() { 0.75 } else { 0.0 };
        download(agent, &format!("{BUILDBOT}/nightly/windows/x86_64/latest/{}.dll.zip", core.file), &zip, |f| {
            send(&format!("Downloading the {}", core.name), base + f * (0.9 - base))
        })?;
        std::fs::create_dir_all(ra_dir().join("cores")).map_err(|e| e.to_string())?;
        unzip(&zip, &ra_dir().join("cores"))?;
        let _ = std::fs::remove_file(&zip);
    }
    // 3. Files the core needs (Dolphin's Sys folder, LRPS2 resources).
    if let Some(z) = core.system_zip {
        if !marker(z).is_file() {
            let zip = root.join("downloads").join(z);
            download(agent, &format!("{BUILDBOT}/assets/system/{z}"), &zip, |f| send("Downloading system files", 0.9 + f * 0.08))?;
            std::fs::create_dir_all(root.join("system")).map_err(|e| e.to_string())?;
            unzip(&zip, &root.join("system"))?;
            let _ = std::fs::remove_file(&zip);
            let _ = std::fs::write(marker(z), "ok");
        }
    }
    send("Ready", 1.0);
    Ok(())
}

/// Player settings: fullscreen, no menu clutter, quit when the game closes.
/// RetroAchievements sign-in is passed along so cores unlock achievements too.
pub fn write_config(ra_user: &str, ra_token: &str) -> PathBuf {
    let r = root();
    let fwd = |p: PathBuf| p.to_string_lossy().replace('\\', "/");
    for d in ["system", "saves", "states", "screenshots"] {
        let _ = std::fs::create_dir_all(r.join(d));
    }
    let cfg = format!(
        r#"video_fullscreen = "true"
video_windowed_fullscreen = "true"
video_driver = "glcore"
video_vsync = "true"
menu_driver = "rgui"
pause_nonactive = "false"
config_save_on_exit = "false"
quit_press_twice = "false"
quit_on_close_content = "1"
input_menu_toggle_gamepad_combo = "2"
input_exit_emulator = "escape"
input_joypad_driver = "xinput"
input_autodetect_enable = "true"
video_font_enable = "false"
notification_show_autoconfig = "false"
notification_show_config_override_load = "false"
notification_show_remap_load = "false"
notification_show_patch_applied = "false"
menu_show_load_content_animation = "false"
savestate_auto_save = "false"
system_directory = "{}"
savefile_directory = "{}"
savestate_directory = "{}"
screenshot_directory = "{}"
"#,
        fwd(r.join("system")),
        fwd(r.join("saves")),
        fwd(r.join("states")),
        fwd(r.join("screenshots")),
    );
    let cfg = if !ra_user.is_empty() && !ra_token.is_empty() {
        format!("{cfg}cheevos_enable = \"true\"\ncheevos_username = \"{ra_user}\"\ncheevos_token = \"{ra_token}\"\n")
    } else {
        cfg
    };
    let p = r.join("nxe-player.cfg");
    let _ = std::fs::write(&p, cfg);
    p
}
