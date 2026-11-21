//! GitHub releases: the Get Ports page (install/update native PC ports) and NXE Launcher's
//! own update check. One worker thread does all the downloading.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

pub const SELF_REPO: &str = "cinarsavas/nxe-launcher";

/// A port you can install from the Get Ports page.
pub struct PortInfo {
    pub name: &'static str,
    pub repo: &'static str,
    pub folder: &'static str,
    pub about: &'static str,
    /// What you need to bring yourself.
    pub needs: &'static str,
}

/// Every repository here was checked to exist and to publish tagged releases.
pub const PORTS: &[PortInfo] = &[
    PortInfo {
        name: "Zelda 64: Recompiled",
        repo: "Zelda64Recomp/Zelda64Recomp",
        folder: "Zelda64Recomp",
        about: "Majora's Mask, recompiled to run natively: widescreen, high frame rates, mods.",
        needs: "Your own Majora's Mask N64 ROM (US). The game asks for it the first time it starts.",
    },
    PortInfo {
        name: "Ship of Harkinian",
        repo: "HarbourMasters/Shipwright",
        folder: "Shipwright",
        about: "Ocarina of Time as a PC port, with enhancements and randomizer.",
        needs: "Your own Ocarina of Time N64 ROM. It asks for it on first start.",
    },
    PortInfo {
        name: "2 Ship 2 Harkinian",
        repo: "HarbourMasters/2ship2harkinian",
        folder: "2Ship2Harkinian",
        about: "Majora's Mask as a PC port from the Ship of Harkinian team.",
        needs: "Your own Majora's Mask N64 ROM (US). It asks for it on first start.",
    },
    PortInfo {
        name: "Starship",
        repo: "HarbourMasters/Starship",
        folder: "Starship",
        about: "Star Fox 64 as a PC port.",
        needs: "Your own Star Fox 64 N64 ROM (US 1.1). It asks for it on first start.",
    },
    PortInfo {
        name: "Spaghetti Kart",
        repo: "HarbourMasters/SpaghettiKart",
        folder: "SpaghettiKart",
        about: "Mario Kart 64 as a PC port.",
        needs: "Your own Mario Kart 64 N64 ROM (US). It asks for it on first start.",
    },
    PortInfo {
        name: "Banjo: Recompiled",
        repo: "BanjoRecomp/BanjoRecomp",
        folder: "BanjoRecomp",
        about: "Banjo-Kazooie, recompiled to run natively.",
        needs: "Your own Banjo-Kazooie N64 ROM. The game asks for it the first time it starts.",
    },
    PortInfo {
        name: "Goemon 64: Recompiled",
        repo: "klorfmorf/Goemon64Recomp",
        folder: "Goemon64Recomp",
        about: "Mystical Ninja Starring Goemon, recompiled. Still in development.",
        needs: "Your own Mystical Ninja Starring Goemon N64 ROM.",
    },
    PortInfo {
        name: "Super Mario 64 Co-op DX",
        repo: "coop-deluxe/sm64coopdx",
        folder: "sm64coopdx",
        about: "Super Mario 64 as a PC port with online co-op.",
        needs: "Your own Super Mario 64 N64 ROM (US). It asks for it on first start.",
    },
    PortInfo {
        name: "Unleashed Recompiled",
        repo: "hedge-dev/UnleashedRecomp",
        folder: "UnleashedRecomp",
        about: "Sonic Unleashed (Xbox 360), recompiled to run natively.",
        needs: "Your own Sonic Unleashed Xbox 360 game files. The installer inside the game asks for them.",
    },
    PortInfo {
        name: "Marathon Recompiled",
        repo: "sonicnext-dev/MarathonRecomp",
        folder: "MarathonRecomp",
        about: "Sonic the Hedgehog (2006, Xbox 360), recompiled to run natively.",
        needs: "Your own Sonic the Hedgehog (2006) Xbox 360 game files. The game's installer asks for them.",
    },
    PortInfo {
        name: "Ghostship",
        repo: "HarbourMasters/Ghostship",
        folder: "Ghostship",
        about: "Super Mario 64 as a PC port from the Ship of Harkinian team.",
        needs: "Your own Super Mario 64 N64 ROM (US). It asks for it on first start.",
    },
    PortInfo {
        name: "Perfect Dark",
        repo: "fgsfdsfgs/perfect_dark",
        folder: "PerfectDark",
        about: "Perfect Dark as a native PC port, with mouse aim and high frame rates.",
        needs: "Your own Perfect Dark N64 ROM (US v1.1), placed in the port's data folder.",
    },
    PortInfo {
        name: "Sonic 3 A.I.R.",
        repo: "Eukaryot/sonic3air",
        folder: "Sonic3AIR",
        about: "Sonic 3 & Knuckles remastered: widescreen, many improvements and mods.",
        needs: "Your own Sonic 3 & Knuckles ROM (it can use the one from the Steam release).",
    },
    PortInfo {
        name: "Sonic Mania (RSDKv5)",
        repo: "RSDKModding/RSDKv5-Decompilation",
        folder: "RSDKv5",
        about: "Decompiled engine of Sonic Mania, runs the game natively.",
        needs: "Data.rsdk from your own copy of Sonic Mania, placed next to the program.",
    },
    PortInfo {
        name: "Sonic 1 & 2 (RSDKv4)",
        repo: "RSDKModding/RSDKv4-Decompilation",
        folder: "RSDKv4",
        about: "Decompiled engine of the 2013 Sonic 1 and Sonic 2 remasters.",
        needs: "Data.rsdk from your own copy of the remasters, placed next to the program.",
    },
    PortInfo {
        name: "Zelda 3",
        repo: "snesrev/zelda3",
        folder: "Zelda3",
        about: "A Link to the Past reimplemented in C: widescreen and enhancements.",
        needs: "Your own A Link to the Past SNES ROM (US), named zelda3.sfc.",
    },
    PortInfo {
        name: "Dusk",
        repo: "TwilitRealm/dusk",
        folder: "Dusk",
        about: "Twilight Princess (GameCube) as a PC port.",
        needs: "Your own Twilight Princess GameCube disc image. It asks for it on first start.",
    },
];

#[derive(Clone, Debug)]
pub struct Release {
    pub tag: String,
    /// (file name, download URL, size in bytes)
    pub assets: Vec<(String, String, u64)>,
}

impl Release {
    /// The Windows download, if the release has one.
    pub fn windows_asset(&self) -> Option<&(String, String, u64)> {
        pick_windows(&self.assets)
    }
}

pub fn pick_windows(assets: &[(String, String, u64)]) -> Option<&(String, String, u64)> {
    let bad = ["linux", "mac", "osx", "darwin", "appimage", "android", "ios", "flatpak", "debug", "pdb", "symbols", "arm64", "aarch64", "source", ".sha", ".sig", ".asc", ".txt", ".json"];
    let ok_ext = |n: &str| n.ends_with(".zip") || n.ends_with(".7z") || n.ends_with(".exe");
    let mut best: Option<(&(String, String, u64), i32)> = None;
    for a in assets {
        let n = a.0.to_lowercase();
        if !ok_ext(&n) || bad.iter().any(|b| n.contains(b)) {
            continue;
        }
        let mut score = 0;
        if n.contains("win") {
            score += 10;
        }
        if n.contains("x64") || n.contains("64") {
            score += 2;
        }
        if n.ends_with(".zip") {
            score += 1;
        }
        if n.contains("installer") || n.contains("setup") {
            score -= 5;
        }
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((a, score));
        }
    }
    best.map(|(a, _)| a)
}

/// "v1.2.3" / "1.2" / "reindeer-games-2025-1" -> comparable numbers.
pub fn version_key(tag: &str) -> Vec<u64> {
    tag.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
}

pub fn newer(remote: &str, local: &str) -> bool {
    version_key(remote) > version_key(local)
}

pub fn ports_dir() -> PathBuf {
    crate::config::base_dir().join("ports")
}

/// What we installed: ports\<folder>\.nxe-port.json
pub fn installed_tag(p: &PortInfo) -> Option<String> {
    let t = std::fs::read_to_string(ports_dir().join(p.folder).join(".nxe-port.json")).ok()?;
    serde_json::from_str::<serde_json::Value>(&t).ok()?["tag"].as_str().map(String::from)
}

pub enum Job {
    /// Look up the latest release of these repos.
    Check(Vec<&'static str>),
    InstallPort(&'static PortInfo, Release),
    /// Download and swap in a new NXE Launcher.
    UpdateSelf(Release),
    /// Pictures for a game or port page.
    Pictures { key: String, system: crate::systems::System, name: String, stem: String, readme: Option<&'static str> },
}

pub enum Msg {
    Latest(&'static str, Result<Release, String>),
    Progress(&'static str, f32),
    Installed(&'static str, Result<String, String>),
    SelfReady(Result<PathBuf, String>),
    /// Encoded images (PNG/JPG) for a page; empty = none found.
    Pictures(String, Vec<Vec<u8>>),
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent(concat!("nxe-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

pub fn latest(agent: &ureq::Agent, repo: &str) -> Result<Release, String> {
    let mut r = agent
        .get(&format!("{}/repos/{repo}/releases/latest", std::env::var("NXE_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".into())))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    let status = r.status().as_u16();
    let text = r.body_mut().read_to_string().map_err(|e| e.to_string())?;
    if status == 404 {
        // Some projects only publish pre-releases (betas, rolling builds): take the newest one.
        let base = std::env::var("NXE_GITHUB_API").unwrap_or_else(|_| "https://api.github.com".into());
        let mut r = agent.get(&format!("{base}/repos/{repo}/releases?per_page=5")).header("Accept", "application/vnd.github+json").call().map_err(|e| e.to_string())?;
        if r.status().as_u16() != 200 {
            return Err("No releases yet".into());
        }
        let v: serde_json::Value = serde_json::from_str(&r.body_mut().read_to_string().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        return v.as_array().and_then(|a| a.iter().find(|x| x["draft"] != true)).map(parse_release).ok_or_else(|| "No releases yet".into());
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if status != 200 {
        // e.g. "API rate limit exceeded"
        return Err(v["message"].as_str().unwrap_or("GitHub didn't answer").to_string());
    }
    Ok(parse_release(&v))
}

pub fn parse_release(v: &serde_json::Value) -> Release {
    Release {
        tag: v["tag_name"].as_str().unwrap_or_default().to_string(),
        assets: v["assets"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| Some((x["name"].as_str()?.to_string(), x["browser_download_url"].as_str()?.to_string(), x["size"].as_u64().unwrap_or(0))))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Unpacks into `dest`; if everything sits in one top folder, that folder's contents are used.
fn unpack(archive: &Path, dest: &Path) -> Result<(), String> {
    let tmp = dest.with_extension("unpacking");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let n = archive.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    if n.ends_with(".zip") {
        crate::runtime::unzip(archive, &tmp)?;
    } else if n.ends_with(".7z") {
        sevenz_rust2::decompress_file(archive, &tmp).map_err(|e| format!("unpack failed: {e}"))?;
    } else {
        std::fs::copy(archive, tmp.join(archive.file_name().unwrap())).map_err(|e| e.to_string())?;
    }
    let entries: Vec<PathBuf> = std::fs::read_dir(&tmp).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
    let src = if entries.len() == 1 && entries[0].is_dir() { entries[0].clone() } else { tmp.clone() };
    // Updating: keep the port's own settings/saves; new files replace old ones.
    copy_tree(&src, dest)?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            copy_tree(&p, &t)?;
        } else {
            std::fs::copy(&p, &t).map_err(|err| format!("{}: {err}", t.to_string_lossy()))?;
        }
    }
    Ok(())
}

pub fn spawn(rx: Receiver<Job>, tx: Sender<Msg>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let agent = agent();
        while let Ok(job) = rx.recv() {
            match job {
                Job::Check(repos) => {
                    for r in repos {
                        let _ = tx.send(Msg::Latest(r, latest(&agent, r)));
                        ctx.request_repaint();
                    }
                }
                Job::Pictures { key, system, name, stem, readme } => {
                    let mut urls = crate::covers::picture_urls(crate::theme_dir(), system, &name, &stem);
                    if let Some(repo) = readme {
                        let md = agent
                            .get(&format!("https://raw.githubusercontent.com/{repo}/HEAD/README.md"))
                            .call()
                            .ok()
                            .filter(|r| r.status() == 200)
                            .and_then(|mut r| r.body_mut().read_to_string().ok())
                            .unwrap_or_default();
                        urls.extend(crate::covers::readme_images(repo, &md));
                    }
                    let mut imgs = vec![];
                    for u in urls.iter().take(6) {
                        let Ok(mut r) = agent.get(u).call() else { continue };
                        if r.status() != 200 {
                            continue;
                        }
                        let Ok(b) = r.body_mut().with_config().limit(8 << 20).read_to_vec() else { continue };
                        // Skip anything that isn't a real picture or is tiny (icons).
                        if let Ok(img) = image::load_from_memory(&b) {
                            if img.width() >= 160 && img.height() >= 100 {
                                imgs.push(b);
                            }
                        }
                    }
                    let _ = tx.send(Msg::Pictures(key, imgs));
                    ctx.request_repaint();
                }
                Job::InstallPort(p, rel) => {
                    let res = (|| {
                        let (name, url, _) = rel.windows_asset().ok_or("This release has no Windows download")?.clone();
                        let _ = std::fs::create_dir_all(ports_dir());
                        let arc = ports_dir().join(format!(".download-{name}"));
                        let tx2 = tx.clone();
                        let c2 = ctx.clone();
                        crate::runtime::download(&agent, &url, &arc, move |f| {
                            let _ = tx2.send(Msg::Progress(p.repo, f));
                            c2.request_repaint();
                        })?;
                        let dest = ports_dir().join(p.folder);
                        let r = unpack(&arc, &dest);
                        let _ = std::fs::remove_file(&arc);
                        r?;
                        let meta = serde_json::json!({ "repo": p.repo, "tag": rel.tag, "asset": name });
                        let _ = std::fs::write(dest.join(".nxe-port.json"), meta.to_string());
                        Ok(rel.tag.clone())
                    })();
                    let _ = tx.send(Msg::Installed(p.repo, res));
                    ctx.request_repaint();
                }
                Job::UpdateSelf(rel) => {
                    let res = (|| {
                        let (_, url, _) = rel
                            .assets
                            .iter()
                            .find(|a| a.0.to_lowercase().ends_with("windows-x64.zip"))
                            .ok_or("The new release has no Windows download yet. Try again in a few minutes.")?
                            .clone();
                        let base = crate::config::base_dir();
                        let zip_path = base.join(".nxe-update.zip");
                        let tx2 = tx.clone();
                        let c2 = ctx.clone();
                        crate::runtime::download(&agent, &url, &zip_path, move |f| {
                            let _ = tx2.send(Msg::Progress(SELF_REPO, f));
                            c2.request_repaint();
                        })?;
                        let f = std::fs::File::open(&zip_path).map_err(|e| e.to_string())?;
                        let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
                        let idx = (0..z.len())
                            .find(|i| z.by_index(*i).map(|e| e.name().map(|n| n.to_lowercase().ends_with("nxe-launcher.exe")).unwrap_or(false)).unwrap_or(false))
                            .ok_or("nxe-launcher.exe is missing from the download")?;
                        let new_exe = base.join("nxe-launcher.new.exe");
                        let mut out = std::fs::File::create(&new_exe).map_err(|e| e.to_string())?;
                        std::io::copy(&mut z.by_index(idx).map_err(|e| e.to_string())?, &mut out).map_err(|e| e.to_string())?;
                        drop(out);
                        let _ = std::fs::remove_file(&zip_path);
                        Ok(new_exe)
                    })();
                    let _ = tx.send(Msg::SelfReady(res));
                    ctx.request_repaint();
                }
            }
        }
    });
}

/// Swap in the downloaded exe and restart. Windows lets a running exe be renamed, not replaced.
pub fn apply_self_update(new_exe: &Path) -> Result<(), String> {
    let cur = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = cur.with_file_name("nxe-launcher.old.exe");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&cur, &old).map_err(|e| format!("Couldn't replace the launcher: {e}"))?;
    if let Err(e) = std::fs::rename(new_exe, &cur) {
        let _ = std::fs::rename(&old, &cur);
        return Err(e.to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&cur, std::fs::Permissions::from_mode(0o755));
    }
    if let Err(e) = std::process::Command::new(&cur).current_dir(cur.parent().unwrap_or(Path::new("."))).spawn() {
        // Put the old version back so nothing is broken.
        let _ = std::fs::remove_file(&cur);
        let _ = std::fs::rename(&old, &cur);
        return Err(e.to_string());
    }
    std::process::exit(0);
}

/// Leftover from the last update.
pub fn cleanup_old() {
    if let Ok(cur) = std::env::current_exe() {
        let _ = std::fs::remove_file(cur.with_file_name("nxe-launcher.old.exe"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_windows_and_versions() {
        let a = |n: &str| (n.to_string(), format!("https://x/{n}"), 1);
        let assets = vec![a("Zelda64Recompiled-v1.2.2-Linux-X64.zip"), a("Zelda64Recompiled-v1.2.2-Windows.zip"), a("Zelda64Recompiled-v1.2.2-macOS.dmg")];
        assert_eq!(pick_windows(&assets).unwrap().0, "Zelda64Recompiled-v1.2.2-Windows.zip");
        let b = vec![a("SoH-Win64.zip"), a("SoH-Win64-pdb.zip"), a("SoH-Linux.AppImage")];
        assert_eq!(pick_windows(&b).unwrap().0, "SoH-Win64.zip");
        assert!(newer("v0.10.0", "0.9.0"));
        assert!(!newer("v0.9.0", "0.9.0"));
        assert!(newer("v1.2.3", "v1.2.2"));
    }
}
