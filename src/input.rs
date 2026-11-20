//! Gamepad input on its own thread so the Guide / Back+Start shortcut keeps
//! working while a game has focus.

use gilrs::{Axis, Button, EventType, Gilrs};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

pub fn spawn(tx: Sender<Pad>, ctx: eframe::egui::Context, shared: Arc<Shared>) {
    std::thread::spawn(move || {
        let Ok(mut gilrs) = Gilrs::new() else {
            eprintln!("gamepad support unavailable");
            return;
        };
        let mut held: Option<(Pad, Instant, Instant)> = None;
        let mut combo_since: Option<Instant> = None;
        loop {
            let running = shared.running.load(Ordering::Relaxed);
            while let Some(ev) = gilrs.next_event() {
                if let EventType::ButtonPressed(b, _) = ev.event {
                    let p = match b {
                        Button::South => Some(Pad::A),
                        Button::East => Some(Pad::B),
                        Button::North => Some(Pad::Y),
                        Button::West => Some(Pad::X),
                        Button::LeftTrigger => Some(Pad::Lb),
                        Button::RightTrigger => Some(Pad::Rb),
                        Button::Start => Some(Pad::Start),
                        Button::Mode => Some(Pad::Guide),
                        _ => None,
                    };
                    if let Some(p) = p {
                        if running {
                            if p == Pad::Guide {
                                let sh = shared.clone();
                                std::thread::spawn(move || kill(&sh));
                            }
                        } else {
                            let _ = tx.send(p);
                            ctx.request_repaint();
                        }
                    }
                }
            }

            // Back + Start held for 1.5 s while playing = return to the dashboard.
            let combo = gilrs
                .gamepads()
                .any(|(_, g)| g.is_pressed(Button::Select) && g.is_pressed(Button::Start));
            if running && combo {
                let since = *combo_since.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_millis(1500) {
                    let sh = shared.clone();
                    std::thread::spawn(move || kill(&sh));
                    combo_since = Some(Instant::now() + Duration::from_secs(5));
                }
            } else {
                combo_since = None;
            }

            // Directions with key-repeat (d-pad or left stick).
            let mut dir = None;
            for (_, g) in gilrs.gamepads() {
                let x = g.value(Axis::LeftStickX);
                let y = g.value(Axis::LeftStickY);
                if g.is_pressed(Button::DPadUp) || y > 0.6 {
                    dir = Some(Pad::Up)
                } else if g.is_pressed(Button::DPadDown) || y < -0.6 {
                    dir = Some(Pad::Down)
                } else if g.is_pressed(Button::DPadLeft) || x < -0.6 {
                    dir = Some(Pad::Left)
                } else if g.is_pressed(Button::DPadRight) || x > 0.6 {
                    dir = Some(Pad::Right)
                }
                if dir.is_some() {
                    break;
                }
            }
            if running {
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
            std::thread::sleep(Duration::from_millis(if running { 30 } else { 10 }));
        }
    });
}

/// Close the running emulator: polite WM_CLOSE first (lets it save), then force after 3 s.
fn kill(shared: &Shared) {
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
