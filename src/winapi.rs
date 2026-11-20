//! Small Win32 helpers for clean handoffs between the dashboard and emulators.
//! On other platforms these are no-ops.

#[cfg(windows)]
mod imp {
    type Hwnd = isize;
    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
        fn IsWindowVisible(h: Hwnd) -> i32;
        fn PostMessageW(h: Hwnd, msg: u32, w: usize, l: isize) -> i32;
        fn SetForegroundWindow(h: Hwnd) -> i32;
        fn BringWindowToTop(h: Hwnd) -> i32;
        fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
        fn IsIconic(h: Hwnd) -> i32;
        fn keybd_event(vk: u8, scan: u8, flags: u32, extra: usize);
    }
    const WM_CLOSE: u32 = 0x0010;
    const SW_RESTORE: i32 = 9;
    const VK_MENU: u8 = 0x12;
    const KEYEVENTF_KEYUP: u32 = 0x0002;

    struct Find {
        pid: u32,
        found: Vec<Hwnd>,
    }

    extern "system" fn collect(h: Hwnd, l: isize) -> i32 {
        let f = unsafe { &mut *(l as *mut Find) };
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        if pid == f.pid && unsafe { IsWindowVisible(h) } != 0 {
            f.found.push(h);
        }
        1
    }

    fn windows_of(pid: u32) -> Vec<Hwnd> {
        let mut f = Find { pid, found: vec![] };
        unsafe { EnumWindows(collect, &mut f as *mut Find as isize) };
        f.found
    }

    /// Ask every visible window of the process to close (like clicking X),
    /// so the emulator can save settings / flush memory cards.
    pub fn request_close(pid: u32) -> bool {
        let w = windows_of(pid);
        for h in &w {
            unsafe { PostMessageW(*h, WM_CLOSE, 0, 0) };
        }
        !w.is_empty()
    }

    /// Pull a process's window to the front. Windows blocks focus stealing,
    /// so tap Alt first (the documented-by-everyone workaround).
    pub fn focus_pid(pid: u32) -> bool {
        let w = windows_of(pid);
        let Some(&h) = w.first() else { return false };
        unsafe {
            if IsIconic(h) != 0 {
                ShowWindow(h, SW_RESTORE);
            }
            keybd_event(VK_MENU, 0, 0, 0);
            keybd_event(VK_MENU, 0, KEYEVENTF_KEYUP, 0);
            BringWindowToTop(h);
            SetForegroundWindow(h);
        }
        true
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn request_close(_pid: u32) -> bool {
        false
    }
    pub fn focus_pid(_pid: u32) -> bool {
        false
    }
}

pub use imp::{focus_pid, request_close};
