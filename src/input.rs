//! Gamepad input on its own thread so the Guide / Back+Start shortcut keeps
//! working while a game has focus.

use gilrs::{Axis, Button, EventType, Gilrs};
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pad {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Y,
    X,
    Lb,
    Rb,
    Start,
    Guide,
}

pub struct Shared {
    pub child: Mutex<Option<Child>>,
    pub running: AtomicBool,
    pub closing: AtomicBool,
    /// The Guide menu is open over a running game: buttons go to the dashboard again.
    pub guide_open: AtomicBool,
}

// ------------------------------------------------------------------ controller styles

/// Which button icons to draw.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    Xbox,
    PlayStation,
    /// PS3: START / SELECT instead of OPTIONS / SHARE.
    PlayStation3,
    Nintendo,
}

/// Last controller that was used (detected).
static DETECTED: AtomicU8 = AtomicU8::new(0);
/// 0 = follow the controller, 1 = Xbox, 2 = PlayStation, 3 = Nintendo (from settings).
pub static OVERRIDE: AtomicU8 = AtomicU8::new(0);
/// Nintendo controllers: confirm with the right button (A), like on a Switch.
pub static SWITCH_LAYOUT: AtomicBool = AtomicBool::new(true);
/// Names of connected controllers, for the Controller settings page.
pub static CONNECTED: Mutex<Vec<(String, Style)>> = Mutex::new(Vec::new());

fn style_from(n: u8) -> Style {
    match n {
        1 => Style::PlayStation,
        2 => Style::PlayStation3,
        3 => Style::Nintendo,
        _ => Style::Xbox,
    }
}

pub fn style() -> Style {
    match OVERRIDE.load(Ordering::Relaxed) {
        1 => Style::Xbox,
        2 => match style_from(DETECTED.load(Ordering::Relaxed)) {
            Style::PlayStation3 => Style::PlayStation3,
            _ => Style::PlayStation,
        },
        3 => Style::Nintendo,
        _ => style_from(DETECTED.load(Ordering::Relaxed)),
    }
}

/// What kind of device a gamepad is (by USB vendor/product ID).
#[derive(Clone, Copy, PartialEq, Debug)]
enum Dev {
    Xbox,
    Ps,
    Ps3,
    Nintendo,
    JoyConL,
    JoyConR,
}

const SONY: u16 = 0x054c;
const NINTENDO: u16 = 0x057e;

fn dev_of(g: &gilrs::Gamepad) -> Dev {
    match (g.vendor_id(), g.product_id()) {
        (Some(SONY), Some(0x0268)) => Dev::Ps3,
        (Some(SONY), _) => Dev::Ps,
        (Some(NINTENDO), Some(0x2006)) => Dev::JoyConL,
        (Some(NINTENDO), Some(0x2007)) => Dev::JoyConR,
        (Some(NINTENDO), _) => Dev::Nintendo,
        _ => {
            // Some drivers hide the IDs; fall back to the name.
            let n = g.name().to_lowercase();
            if n.contains("joy-con (l)") {
                Dev::JoyConL
            } else if n.contains("joy-con (r)") {
                Dev::JoyConR
            } else if n.contains("ps3") || n.contains("playstation(r)3") {
                Dev::Ps3
            } else if n.contains("ps4") || n.contains("ps5") || n.contains("dualshock") || n.contains("dualsense") || n.contains("wireless controller") {
                Dev::Ps
            } else if n.contains("nintendo") || n.contains("switch") || n.contains("pro controller") {
                Dev::Nintendo
            } else {
                Dev::Xbox
            }
        }
    }
}

fn dev_style(d: Dev) -> u8 {
    match d {
        Dev::Xbox => 0,
        Dev::Ps => 1,
        Dev::Ps3 => 2,
        Dev::Nintendo | Dev::JoyConL | Dev::JoyConR => 3,
    }
}

/// Face buttons -> dashboard buttons. gilrs reports buttons by position (South = bottom).
/// `pair`: a Joy-Con (L) and (R) are both connected and held upright as one controller;
/// on their own, Joy-Cons are held sideways.
fn map_button(d: Dev, b: Button, pair: bool) -> Option<Pad> {
    let switch = SWITCH_LAYOUT.load(Ordering::Relaxed);
    let face = |south, east, west, north| match b {
        Button::South => Some(south),
        Button::East => Some(east),
        Button::West => Some(west),
        Button::North => Some(north),
        _ => None,
    };
    let (a, bb, x, y) = (Pad::A, Pad::B, Pad::X, Pad::Y);
    let f = match d {
        // Upright left Joy-Con: its four buttons are the arrows (handled with the stick).
        Dev::JoyConL if pair => return None,
        // Upright right Joy-Con. Sideways mapping: South = A, East = X, West = B, North = Y.
        Dev::JoyConR if pair && switch => face(a, x, bb, y),
        Dev::JoyConR if pair => face(bb, y, a, x),
        Dev::Nintendo | Dev::JoyConL | Dev::JoyConR if switch => face(bb, a, y, x),
        _ => face(a, bb, x, y),
    };
    if f.is_some() {
        return f;
    }
    match (d, b) {
        // Right Joy-Con: "back" in its mapping is the Home button.
        (Dev::JoyConR, Button::Select) => Some(Pad::Guide),
        (_, Button::LeftTrigger) => Some(Pad::Lb),
        (_, Button::RightTrigger) => Some(Pad::Rb),
        (_, Button::Start) => Some(Pad::Start),
        (_, Button::Mode) => Some(Pad::Guide),
        _ => None,
    }
}

/// Direction from one gamepad (d-pad or left stick), turned upright for paired Joy-Cons.
fn direction(d: Dev, g: &gilrs::Gamepad, pair: bool) -> Option<Pad> {
    let (mut x, mut y) = (g.value(Axis::LeftStickX), g.value(Axis::LeftStickY));
    let (mut up, mut down, mut left, mut right) =
        (g.is_pressed(Button::DPadUp), g.is_pressed(Button::DPadDown), g.is_pressed(Button::DPadLeft), g.is_pressed(Button::DPadRight));
    if pair && d == Dev::JoyConL {
        // Mapped for sideways use: turn the stick a quarter back, and the face buttons are arrows.
        (x, y) = (y, -x);
        left |= g.is_pressed(Button::South);
        down |= g.is_pressed(Button::East);
        up |= g.is_pressed(Button::West);
        right |= g.is_pressed(Button::North);
    } else if pair && d == Dev::JoyConR {
        (x, y) = (-y, x);
    }
    if up || y > 0.6 {
        Some(Pad::Up)
    } else if down || y < -0.6 {
        Some(Pad::Down)
    } else if left || x < -0.6 {
        Some(Pad::Left)
    } else if right || x > 0.6 {
        Some(Pad::Right)
    } else {
        None
    }
}

pub fn spawn(tx: Sender<Pad>, ctx: eframe::egui::Context, shared: Arc<Shared>) {
    std::thread::spawn(move || {
        let Ok(mut gilrs) = Gilrs::new() else {
            eprintln!("gamepad support unavailable");
            return;
        };
        let mut held: Option<(Pad, Instant, Instant)> = None;
        let mut combo_since: Option<Instant> = None;
        let mut last_list = Instant::now() - Duration::from_secs(10);
        loop {
            let running = shared.running.load(Ordering::Relaxed);
            let guide = shared.guide_open.load(Ordering::Relaxed);
            let to_app = !running || guide;
            let devs: Vec<(gilrs::GamepadId, Dev)> = gilrs.gamepads().map(|(id, g)| (id, dev_of(&g))).collect();
            let pair = devs.iter().any(|(_, d)| *d == Dev::JoyConL) && devs.iter().any(|(_, d)| *d == Dev::JoyConR);
            if last_list.elapsed() > Duration::from_secs(1) {
                last_list = Instant::now();
                if let Ok(mut c) = CONNECTED.lock() {
                    *c = gilrs.gamepads().map(|(_, g)| (g.name().to_string(), style_from(dev_style(dev_of(&g))))).collect();
                }
            }
            while let Some(ev) = gilrs.next_event() {
                let d = devs.iter().find(|(id, _)| *id == ev.id).map(|(_, d)| *d).unwrap_or(Dev::Xbox);
                if let EventType::ButtonPressed(b, _) = ev.event {
                    DETECTED.store(dev_style(d), Ordering::Relaxed);
                    let Some(p) = map_button(d, b, pair) else { continue };
                    if to_app || p == Pad::Guide {
                        // While playing, only Guide reaches the dashboard (it opens the Guide menu).
                        let _ = tx.send(p);
                        ctx.request_repaint();
                    }
                }
            }

            // Back + Start held for 1 s while playing = Guide menu (for pads without a Guide button).
            let combo = gilrs.gamepads().any(|(_, g)| g.is_pressed(Button::Select) && g.is_pressed(Button::Start));
            if running && !guide && combo {
                let since = *combo_since.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_millis(1000) {
                    let _ = tx.send(Pad::Guide);
                    ctx.request_repaint();
                    combo_since = Some(Instant::now() + Duration::from_secs(5));
                }
            } else {
                combo_since = None;
            }

            // Directions with key-repeat.
            let mut dir = None;
            for (id, d) in &devs {
                if let Some(v) = direction(*d, &gilrs.gamepad(*id), pair) {
                    dir = Some(v);
                    DETECTED.store(dev_style(*d), Ordering::Relaxed);
                    break;
                }
            }
            if !to_app {
                dir = None;
            }
            let now = Instant::now();
            match (dir, held) {
                (Some(d), Some((h, start, last))) if d == h => {
                    if now - start > Duration::from_millis(380) && now - last > Duration::from_millis(110) {
                        let _ = tx.send(d);
                        ctx.request_repaint();
                        held = Some((h, start, now));
                    }
                }
                (Some(d), _) => {
                    let _ = tx.send(d);
                    ctx.request_repaint();
                    held = Some((d, now, now));
                }
                (None, _) => held = None,
            }
            std::thread::sleep(Duration::from_millis(if to_app { 10 } else { 30 }));
        }
    });
}

/// Close the running emulator: polite WM_CLOSE first (lets it save), then force after 3 s.
pub fn kill(shared: &Shared) {
    if shared.closing.swap(true, Ordering::Relaxed) {
        return;
    }
    let pid = shared.child.lock().ok().and_then(|c| c.as_ref().map(|c| c.id()));
    if let Some(pid) = pid {
        if crate::winapi::request_close(pid) {
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(100));
                if !shared.running.load(Ordering::Relaxed) {
                    shared.closing.store(false, Ordering::Relaxed);
                    return;
                }
                if let Ok(mut c) = shared.child.lock() {
                    match c.as_mut().map(|c| c.try_wait()) {
                        Some(Ok(None)) => {}
                        _ => {
                            shared.closing.store(false, Ordering::Relaxed);
                            return;
                        }
                    }
                }
            }
        }
    }
    if let Ok(mut c) = shared.child.lock() {
        if let Some(child) = c.as_mut() {
            let _ = child.kill();
        }
    }
    shared.closing.store(false, Ordering::Relaxed);
}
