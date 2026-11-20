//! Optional UI sounds from theme/sounds/<name>.wav (Windows: winmm PlaySoundW, async).

use std::path::PathBuf;

pub struct Sounds {
    dir: PathBuf,
    enabled: bool,
}

impl Sounds {
    pub fn new(dir: PathBuf, enabled: bool) -> Self {
        Self { dir, enabled }
    }

    /// name: "move", "select", "back", "channel", "launch"
    pub fn play(&self, name: &str) {
        if !self.enabled {
            return;
        }
        let path = self.dir.join(format!("{name}.wav"));
        if path.is_file() {
            play_file(&path);
        }
    }
}

#[cfg(windows)]
fn play_file(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "winmm")]
    extern "system" {
        fn PlaySoundW(psz: *const u16, hmod: *mut core::ffi::c_void, flags: u32) -> i32;
    }
    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_FILENAME: u32 = 0x0002_0000;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        PlaySoundW(wide.as_ptr(), std::ptr::null_mut(), SND_ASYNC | SND_NODEFAULT | SND_FILENAME);
    }
}

#[cfg(not(windows))]
fn play_file(_path: &std::path::Path) {}
