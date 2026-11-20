//! Gamerpics: a built-in set (original designs drawn in code), your
//! RetroAchievements picture, or your own image cropped with a pan/zoom editor.

use crate::input::Pad;
use crate::scene::{hint_bar, text_shadow, BTN_A, BTN_B, BTN_GREY};
use crate::{theme_dir, App};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Mesh, Pos2, Rect, Shape, Stroke, StrokeKind};
use std::time::Instant;

pub const BUILTIN: usize = 18;

#[derive(Clone, Copy)]
enum Sym {
    Star,
    Bolt,
    Heart,
    Diamond,
    Moon,
    Planet,
    Note,
    Crown,
    Pad,
    Paw,
    Cube,
    Flower,
    Rocket,
    Ghost,
    Flame,
    Mountain,
    Wave,
    Ring,
}

const DESIGNS: [(Sym, [u8; 3], [u8; 3]); BUILTIN] = [
    (Sym::Pad, [140, 210, 60], [40, 120, 10]),
    (Sym::Star, [255, 196, 60], [210, 100, 10]),
    (Sym::Bolt, [90, 170, 255], [20, 60, 170]),
    (Sym::Heart, [255, 110, 140], [170, 20, 60]),
    (Sym::Diamond, [120, 230, 230], [20, 120, 150]),
    (Sym::Moon, [80, 90, 170], [20, 20, 70]),
    (Sym::Planet, [250, 150, 70], [140, 50, 120]),
    (Sym::Note, [190, 110, 240], [80, 30, 150]),
    (Sym::Crown, [250, 220, 90], [170, 110, 10]),
    (Sym::Paw, [200, 160, 120], [110, 70, 40]),
    (Sym::Cube, [110, 120, 135], [40, 44, 52]),
    (Sym::Flower, [255, 170, 200], [200, 70, 120]),
    (Sym::Rocket, [70, 200, 255], [30, 70, 140]),
    (Sym::Ghost, [150, 130, 220], [50, 40, 110]),
    (Sym::Flame, [255, 140, 40], [180, 30, 10]),
    (Sym::Mountain, [120, 200, 160], [30, 100, 80]),
    (Sym::Wave, [60, 160, 230], [10, 70, 120]),
    (Sym::Ring, [230, 230, 235], [130, 135, 145]),
];

pub const NAMES: [&str; BUILTIN] = [
    "Controller",
    "Superstar",
    "Lightning",
    "Heart",
    "Diamond",
    "Night Owl",
    "Ringworld",
    "Encore",
    "Royalty",
    "Paw Print",
    "Cube",
    "Bloom",
    "Liftoff",
    "Boo",
    "Ember",
    "Summit",
    "Tide",
    "Halo Ring",
];

fn c3(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn grad(r: Rect, top: Color32, bot: Color32) -> Shape {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.left_bottom(), bot);
    m.colored_vertex(r.right_bottom(), bot);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    Shape::mesh(m)
}

/// Draws built-in gamerpic `n` filling `r`.
pub fn draw_builtin(p: &egui::Painter, r: Rect, n: usize, a: f32) {
    let (sym, top, bot) = DESIGNS[n % BUILTIN];
    let mul = |c: Color32| c.gamma_multiply(a);
    let s = r.width();
    let c = r.center();
    // Base: vertical gradient plus a soft spotlight behind the symbol.
    p.add(grad(r, mul(c3(top)), mul(c3(bot))));
    radial(p, c - vec2(0.0, s * 0.05), s * 0.55, Color32::from_white_alpha((45.0 * a) as u8));
    // Darker corners (vignette) for depth.
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        radial(p, c + vec2(dx * s * 0.5, dy * s * 0.5), s * 0.45, Color32::from_black_alpha((38.0 * a) as u8));
    }
    // Background motif rings.
    p.circle_stroke(c + vec2(s * 0.24, -s * 0.26), s * 0.3, Stroke::new(s * 0.025, Color32::from_white_alpha((22.0 * a) as u8)));
    p.circle_stroke(c + vec2(-s * 0.3, s * 0.3), s * 0.2, Stroke::new(s * 0.02, Color32::from_white_alpha((16.0 * a) as u8)));
    // Soft shadow: a few offset layers fake a blur.
    for (k, al) in [(0.05, 22.0), (0.035, 30.0), (0.02, 40.0)] {
        symbol(p, sym, c + vec2(s * k * 0.5, s * k), s * 0.32, Color32::from_black_alpha((al * a) as u8));
    }
    // Symbol: off-white body, brighter top half (two-tone like a lit object).
    symbol(p, sym, c, s * 0.32, Color32::from_rgba_unmultiplied(232, 232, 236, (255.0 * a) as u8));
    let top_half = p.with_clip_rect(Rect::from_min_max(r.min, pos2(r.right(), c.y - s * 0.02)).intersect(p.clip_rect()));
    symbol(&top_half, sym, c, s * 0.32, Color32::from_white_alpha((255.0 * a) as u8));
    // Curved gloss across the top, like the glossy 360-era icons.
    let gloss = p.with_clip_rect(Rect::from_min_max(r.min, pos2(r.right(), r.top() + s * 0.46)).intersect(p.clip_rect()));
    gloss.circle_filled(pos2(c.x - s * 0.1, r.top() - s * 0.95), s * 1.35, Color32::from_white_alpha((34.0 * a) as u8));
    // Bevel: light top-left edge, dark bottom-right edge.
    let t = (s * 0.018).max(1.0);
    p.line_segment([r.left_top(), r.right_top()], Stroke::new(t, Color32::from_white_alpha((70.0 * a) as u8)));
    p.line_segment([r.left_top(), r.left_bottom()], Stroke::new(t, Color32::from_white_alpha((40.0 * a) as u8)));
    p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(t, Color32::from_black_alpha((60.0 * a) as u8)));
    p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(t, Color32::from_black_alpha((45.0 * a) as u8)));
}

fn radial(p: &egui::Painter, c: Pos2, rad: f32, inner: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(c, inner);
    let n = 32;
    for i in 0..=n {
        let ang = i as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(c + vec2(ang.cos(), ang.sin()) * rad, Color32::TRANSPARENT);
        if i > 0 {
            m.add_triangle(0, i as u32, i as u32 + 1);
        }
    }
    p.add(Shape::mesh(m));
}

fn poly(p: &egui::Painter, pts: Vec<Pos2>, col: Color32) {
    p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
}

fn symbol(p: &egui::Painter, sym: Sym, c: Pos2, r: f32, col: Color32) {
    let v = |x: f32, y: f32| c + vec2(x * r, y * r);
    match sym {
        Sym::Star => {
            let pts: Vec<Pos2> = (0..10)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
                    let rr = if i % 2 == 0 { 1.0 } else { 0.42 };
                    v(a.cos() * rr, a.sin() * rr)
                })
                .collect();
            // Star is concave: draw as a fan of triangles.
            for i in 0..10 {
                poly(p, vec![c, pts[i], pts[(i + 1) % 10]], col);
            }
        }
        Sym::Bolt => {
            poly(p, vec![v(0.15, -1.0), v(-0.55, 0.12), v(0.0, 0.12)], col);
            poly(p, vec![v(0.15, -1.0), v(0.0, 0.12), v(0.12, 0.12), v(0.3, -0.25)], col);
            poly(p, vec![v(-0.15, 1.0), v(0.55, -0.12), v(0.0, -0.12)], col);
            poly(p, vec![v(-0.15, 1.0), v(0.0, -0.12), v(-0.12, -0.12), v(-0.3, 0.25)], col);
        }
        Sym::Heart => {
            p.circle_filled(v(-0.42, -0.25), r * 0.48, col);
            p.circle_filled(v(0.42, -0.25), r * 0.48, col);
            poly(p, vec![v(-0.88, -0.1), v(0.88, -0.1), v(0.0, 0.95)], col);
        }
        Sym::Diamond => {
            poly(p, vec![v(-0.85, -0.3), v(-0.45, -0.75), v(0.45, -0.75), v(0.85, -0.3)], col);
            poly(p, vec![v(-0.85, -0.25), v(0.85, -0.25), v(0.0, 0.95)], col);
        }
        Sym::Moon => {
            p.circle_filled(c, r * 0.85, col);
            // Bite out of the moon in a darker tone of the background.
            p.circle_filled(v(0.42, -0.3), r * 0.7, Color32::from_rgb(40, 45, 110).gamma_multiply(col.a() as f32 / 255.0));
        }
        Sym::Planet => {
            p.circle_filled(c, r * 0.55, col);
            let pts: Vec<Pos2> = (0..=40).map(|i| {
                let a = i as f32 / 40.0 * std::f32::consts::TAU;
                v(a.cos() * 1.0, a.sin() * 0.3 + a.cos() * 0.15)
            }).collect();
            p.add(Shape::line(pts, Stroke::new(r * 0.12, col)));
        }
        Sym::Note => {
            p.circle_filled(v(-0.35, 0.55), r * 0.3, col);
            p.circle_filled(v(0.5, 0.35), r * 0.3, col);
            p.line_segment([v(-0.08, 0.55), v(-0.08, -0.7)], Stroke::new(r * 0.14, col));
            p.line_segment([v(0.77, 0.35), v(0.77, -0.9)], Stroke::new(r * 0.14, col));
            poly(p, vec![v(-0.15, -0.72), v(0.84, -0.95), v(0.84, -0.62), v(-0.15, -0.4)], col);
        }
        Sym::Crown => {
            poly(p, vec![v(-0.85, 0.55), v(0.85, 0.55), v(0.85, -0.1), v(-0.85, -0.1)], col);
            poly(p, vec![v(-0.85, -0.1), v(-0.85, -0.7), v(-0.35, -0.1)], col);
            poly(p, vec![v(-0.4, -0.1), v(0.0, -0.85), v(0.4, -0.1)], col);
            poly(p, vec![v(0.35, -0.1), v(0.85, -0.7), v(0.85, -0.1)], col);
            p.rect_filled(Rect::from_min_max(v(-0.85, 0.65), v(0.85, 0.82)), r * 0.04, col);
        }
        Sym::Pad => {
            p.rect_filled(Rect::from_min_max(v(-0.75, -0.35), v(0.75, 0.25)), r * 0.25, col);
            p.circle_filled(v(-0.6, 0.25), r * 0.32, col);
            p.circle_filled(v(0.6, 0.25), r * 0.32, col);
            let hole = Color32::from_black_alpha((90.0 * col.a() as f32 / 255.0) as u8);
            p.rect_filled(Rect::from_center_size(v(-0.42, -0.05), vec2(r * 0.34, r * 0.1)), 1.0, hole);
            p.rect_filled(Rect::from_center_size(v(-0.42, -0.05), vec2(r * 0.1, r * 0.34)), 1.0, hole);
            p.circle_filled(v(0.38, -0.12), r * 0.07, hole);
            p.circle_filled(v(0.52, 0.02), r * 0.07, hole);
        }
        Sym::Paw => {
            p.circle_filled(v(0.0, 0.35), r * 0.45, col);
            for (x, y) in [(-0.6, -0.2), (-0.25, -0.62), (0.25, -0.62), (0.6, -0.2)] {
                p.circle_filled(v(x, y), r * 0.2, col);
            }
        }
        Sym::Cube => {
            let k = col.a() as f32 / 255.0;
            poly(p, vec![v(0.0, -0.9), v(0.8, -0.45), v(0.0, 0.0), v(-0.8, -0.45)], col);
            poly(p, vec![v(-0.8, -0.45), v(0.0, 0.0), v(0.0, 0.95), v(-0.8, 0.5)], Color32::from_white_alpha((200.0 * k) as u8));
            poly(p, vec![v(0.8, -0.45), v(0.8, 0.5), v(0.0, 0.95), v(0.0, 0.0)], Color32::from_white_alpha((150.0 * k) as u8));
        }
        Sym::Flower => {
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                p.circle_filled(v(a.cos() * 0.5, a.sin() * 0.5), r * 0.36, col);
            }
            p.circle_filled(c, r * 0.3, Color32::from_rgb(255, 210, 90).gamma_multiply(col.a() as f32 / 255.0));
        }
        Sym::Rocket => {
            poly(p, vec![v(0.0, -1.0), v(0.35, -0.45), v(0.35, 0.45), v(-0.35, 0.45), v(-0.35, -0.45)], col);
            poly(p, vec![v(-0.35, 0.05), v(-0.7, 0.6), v(-0.35, 0.45)], col);
            poly(p, vec![v(0.35, 0.05), v(0.35, 0.45), v(0.7, 0.6)], col);
            p.circle_filled(v(0.0, -0.25), r * 0.14, Color32::from_rgb(70, 160, 230).gamma_multiply(col.a() as f32 / 255.0));
            poly(p, vec![v(-0.2, 0.5), v(0.2, 0.5), v(0.0, 0.95)], Color32::from_rgb(255, 170, 60).gamma_multiply(col.a() as f32 / 255.0));
        }
        Sym::Ghost => {
            p.circle_filled(v(0.0, -0.2), r * 0.62, col);
            p.rect_filled(Rect::from_min_max(v(-0.62, -0.2), v(0.62, 0.65)), 0.0, col);
            for i in 0..3 {
                let x = -0.62 + i as f32 * 0.42 + 0.21;
                p.circle_filled(v(x, 0.65), r * 0.21, col);
            }
            let eye = Color32::from_rgb(60, 50, 120).gamma_multiply(col.a() as f32 / 255.0);
            p.circle_filled(v(-0.22, -0.25), r * 0.12, eye);
            p.circle_filled(v(0.22, -0.25), r * 0.12, eye);
        }
        Sym::Flame => {
            p.circle_filled(v(0.0, 0.35), r * 0.55, col);
            poly(p, vec![v(-0.55, 0.3), v(0.0, -1.0), v(0.55, 0.3)], col);
            p.circle_filled(v(0.0, 0.45), r * 0.25, Color32::from_rgb(255, 210, 90).gamma_multiply(col.a() as f32 / 255.0));
        }
        Sym::Mountain => {
            poly(p, vec![v(-1.0, 0.7), v(-0.25, -0.6), v(0.45, 0.7)], col);
            poly(p, vec![v(-0.1, 0.7), v(0.45, -0.2), v(1.0, 0.7)], col);
            p.circle_filled(v(0.6, -0.65), r * 0.18, col);
        }
        Sym::Wave => {
            for k in 0..3 {
                let y0 = -0.45 + k as f32 * 0.45;
                let pts: Vec<Pos2> = (0..=30).map(|i| {
                    let x = -0.9 + 1.8 * i as f32 / 30.0;
                    v(x, y0 + (x * 5.0).sin() * 0.12)
                }).collect();
                p.add(Shape::line(pts, Stroke::new(r * 0.16, col)));
            }
        }
        Sym::Ring => {
            p.circle_stroke(c, r * 0.65, Stroke::new(r * 0.22, col));
            p.circle_filled(c, r * 0.18, col);
        }
    }
}

// ---------------------------------------------------------------- picker

pub struct PicPicker {
    pub sel: usize,
    pub opened: Instant,
}

/// Picker entries: built-ins, then pictures from theme\gamerpics, then "your own picture",
/// then the RetroAchievements picture if signed in.
fn entries(app: &App) -> usize {
    BUILTIN + app.user_pics.len() + 2
}

#[derive(PartialEq)]
enum Entry {
    Builtin(usize),
    File(usize),
    Own,
    Ra,
}

fn entry(app: &App, i: usize) -> Entry {
    let f = app.user_pics.len();
    if i < BUILTIN {
        Entry::Builtin(i)
    } else if i < BUILTIN + f {
        Entry::File(i - BUILTIN)
    } else if i == BUILTIN + f {
        Entry::Own
    } else {
        Entry::Ra
    }
}

/// Square, centre-cropped pictures dropped into theme\gamerpics (name = file name).
pub fn load_user_pics(ctx: &egui::Context) -> Vec<(String, egui::TextureHandle)> {
    let dir = theme_dir().join("gamerpics");
    let _ = std::fs::create_dir_all(&dir);
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp")).unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let img = image::open(&p).ok()?;
            let side = img.width().min(img.height());
            let img = img.crop_imm((img.width() - side) / 2, (img.height() - side) / 2, side, side).thumbnail(256, 256).to_rgba8();
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            let name = p.file_stem()?.to_string_lossy().replace(['_', '-'], " ");
            let key = p.file_name()?.to_string_lossy().to_string();
            Some((format!("{key}\u{0}{name}"), ctx.load_texture(format!("upic_{key}"), ci, egui::TextureOptions::LINEAR)))
        })
        .collect()
}

/// Tile shown for the RetroAchievements picture before it's available.
fn ra_placeholder(p: &egui::Painter, r: Rect, a: f32, u: f32) {
    p.add(grad(r, Color32::from_rgb(40, 52, 92).gamma_multiply(a), Color32::from_rgb(14, 18, 38).gamma_multiply(a)));
    crate::scene::icon_pub(p, crate::Icon::Trophy, r.center() - vec2(0.0, r.height() * 0.08), r.width() * 0.26, Color32::from_rgb(240, 200, 80).gamma_multiply(a));
    let fs = (r.width() * 0.12).clamp(9.0 * u, 18.0 * u);
    p.text(pos2(r.center().x, r.bottom() - r.height() * 0.16), Align2::CENTER_CENTER, "Sign In", FontId::proportional(fs), Color32::WHITE.gamma_multiply(a));
}

fn split_key(k: &str) -> (&str, &str) {
    k.split_once('\u{0}').unwrap_or((k, k))
}

/// Display name of the current gamerpic.
pub fn current_name(app: &App) -> String {
    let g = app.cfg.gamerpic.as_str();
    if g == "custom" {
        return "Your picture".into();
    }
    if g == "ra" {
        return "RetroAchievements picture".into();
    }
    if let Some(f) = g.strip_prefix("file:") {
        return app.user_pics.iter().map(|(k, _)| split_key(k)).find(|(key, _)| *key == f).map(|(_, n)| n.to_string()).unwrap_or_else(|| f.to_string());
    }
    let n: usize = g.strip_prefix("builtin:").and_then(|n| n.parse().ok()).unwrap_or(0);
    NAMES[n % BUILTIN].to_string()
}

impl App {
    pub fn open_pic_picker(&mut self) {
        if let Some(ctx) = self.ctx().cloned() {
            self.user_pics = load_user_pics(&ctx);
        }
        let f = self.user_pics.len();
        let g = self.cfg.gamerpic.clone();
        let sel = match g.as_str() {
            "custom" => BUILTIN + f,
            "ra" => BUILTIN + f + 1,
            s if s.starts_with("file:") => {
                let key = &s[5..];
                BUILTIN + self.user_pics.iter().position(|(k, _)| split_key(k).0 == key).unwrap_or(0)
            }
            s => s.strip_prefix("builtin:").and_then(|n| n.parse().ok()).unwrap_or(0),
        };
        self.pic_picker = Some(PicPicker { sel, opened: Instant::now() });
        self.sounds.play("select");
    }

    pub fn pic_picker_input(&mut self, p: Pad, frame: &eframe::Frame, ctx: &egui::Context) {
        let n = entries(self);
        let cols = 6;
        let Some(pk) = self.pic_picker.as_mut() else { return };
        match p {
            Pad::Left if pk.sel % cols > 0 => pk.sel -= 1,
            Pad::Right if pk.sel + 1 < n && pk.sel % cols < cols - 1 => pk.sel += 1,
            Pad::Up if pk.sel >= cols => pk.sel -= cols,
            Pad::Down if pk.sel + cols < n => pk.sel += cols,
            Pad::Down if pk.sel / cols < (n - 1) / cols => pk.sel = n - 1,
            Pad::B => {
                self.pic_picker = None;
                self.sounds.play("back");
                return;
            }
            Pad::A => {
                let sel = pk.sel;
                self.sounds.play("select");
                match entry(self, sel) {
                    Entry::Builtin(i) => {
                        self.cfg.gamerpic = format!("builtin:{i}");
                        self.toast(format!("Gamerpic: {}", NAMES[i]));
                    }
                    Entry::File(i) => {
                        let (key, name) = split_key(&self.user_pics[i].0);
                        let (key, name) = (key.to_string(), name.to_string());
                        self.cfg.gamerpic = format!("file:{key}");
                        self.toast(format!("Gamerpic: {name}"));
                    }
                    Entry::Own => {
                        self.pic_picker = None;
                        self.choose_custom_picture(frame, ctx);
                        return;
                    }
                    Entry::Ra if self.ra_avatar().is_some() => {
                        self.cfg.gamerpic = "ra".into();
                        self.toast("Gamerpic: your RetroAchievements picture");
                    }
                    Entry::Ra => {
                        // Not signed in (or picture not loaded yet): go sign in, then it's applied.
                        self.ra_pic_pending = true;
                        self.pic_picker = None;
                        if self.ra_signed_in() {
                            self.ra_refresh();
                            self.toast("Loading your RetroAchievements picture…");
                        } else if self.menu.as_ref().map(|m| m.kind == crate::menu::Kind::Setup).unwrap_or(false) {
                            self.toast("Sign in under Settings > Achievements after setup");
                        } else {
                            self.menu = Some(crate::menu::Menu::achievements());
                            self.toast("Sign in and your RetroAchievements picture is used");
                        }
                        return;
                    }
                }
                self.cfg.save();
                self.pic_picker = None;
                return;
            }
            _ => return,
        }
        self.sounds.play("move");
    }

    pub fn choose_custom_picture(&mut self, frame: &eframe::Frame, ctx: &egui::Context) {
        let test = std::env::var("NXE_TEST_PIC").ok().map(std::path::PathBuf::from);
        let Some(path) = test.or_else(|| rfd::FileDialog::new()
            .set_title("Choose a picture")
            .add_filter("Pictures", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
            .set_parent(frame)
            .pick_file())
        else {
            return;
        };
        match image::open(&path) {
            Ok(img) => {
                // Keep editing fast: big photos are scaled down first.
                let img = if img.width().max(img.height()) > 2048 { img.thumbnail(2048, 2048) } else { img };
                let rgba = img.to_rgba8();
                let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
                let tex = ctx.load_texture("crop_src", ci, egui::TextureOptions::LINEAR);
                self.cropper = Some(Cropper { img: rgba, tex, zoom: 1.0, cx: 0.5, cy: 0.5, opened: Instant::now(), drag: None });
            }
            Err(e) => self.toast(format!("Couldn't open that picture: {e}")),
        }
    }
}

pub fn draw_pic_picker(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(pk) = app.pic_picker.as_ref() else { return };
    let sel = pk.sel;
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let a = (pk.opened.elapsed().as_secs_f32() / 0.22).min(1.0);
    let f = crate::menu::frame360(&p, app, screen, "Choose a Gamerpic", Some("Profile"), a, true);
    let u = f.u;
    let n = entries(app);
    let cols = 6;
    let rows_needed = n.div_ceil(cols).max(1) as f32;
    let gap = 14.0 * u;
    let cell = (76.0 * u).min((f.panel.height() - 56.0 * u - (rows_needed - 1.0) * gap) / rows_needed);
    let grid_w = cols as f32 * cell + (cols - 1) as f32 * gap;
    let x0 = f.panel.left() + 30.0 * u;
    let y0 = f.panel.top() + 28.0 * u;
    for i in 0..n {
        let (cx, cy) = (i % cols, i / cols);
        let r = Rect::from_min_size(pos2(x0 + cx as f32 * (cell + gap), y0 + cy as f32 * (cell + gap)), vec2(cell, cell));
        if i == sel {
            let pulse = 0.75 + 0.25 * (app.time() * 4.0).sin().abs();
            p.rect_filled(r.expand(6.0 * u), CornerRadius::same((8.0 * u) as u8), Color32::from_rgb(140, 210, 60).gamma_multiply(a * pulse));
        }
        match entry(app, i) {
            Entry::Builtin(n) => draw_builtin(&p.with_clip_rect(r), r, n, a),
            Entry::File(n) => {
                p.image(app.user_pics[n].1.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
            }
            Entry::Own => {
                p.rect_filled(r, 4.0 * u, Color32::from_white_alpha((30.0 * a) as u8));
                if let Some(t) = app.custom_pic() {
                    p.image(t, r.shrink(10.0 * u), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a * 0.6));
                }
                crate::scene::icon_pub(&p, crate::Icon::Plus, r.center() - vec2(0.0, 8.0 * u), cell * 0.2, Color32::WHITE.gamma_multiply(a));
                p.text(pos2(r.center().x, r.bottom() - 14.0 * u), Align2::CENTER_CENTER, "Your Picture", FontId::proportional(13.0 * u), Color32::WHITE.gamma_multiply(a));
            }
            Entry::Ra => {
                if let Some(t) = app.ra_avatar() {
                    p.image(t, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
                } else {
                    ra_placeholder(&p, r, a, u);
                }
            }
        }
        p.rect_stroke(r, 3.0 * u, Stroke::new(1.0, Color32::from_white_alpha((50.0 * a) as u8)), StrokeKind::Inside);
    }
    let (title, label) = match entry(app, sel) {
        Entry::Builtin(n) => (NAMES[n].to_string(), format!("Built-in gamerpic {} of {}", n + 1, BUILTIN)),
        Entry::File(n) => (split_key(&app.user_pics[n].0).1.to_string(), "From your theme\\gamerpics folder".into()),
        Entry::Own => ("Your Picture".into(), "Choose any image, then move and zoom it to fit.".into()),
        Entry::Ra if app.ra_avatar().is_some() => ("RetroAchievements".into(), format!("The profile picture of {} on retroachievements.org.", app.cfg.ra_user)),
        Entry::Ra => (
            "RetroAchievements".into(),
            "Use your retroachievements.org profile picture. Press A to sign in first; the picture is used as soon as you're signed in.".into(),
        ),
    };
    let tx = x0 + grid_w + 30.0 * u;
    let side = (f.panel.right() - tx - 30.0 * u).min(170.0 * u);
    let big = Rect::from_min_size(pos2(tx, y0), vec2(side, side));
    match entry(app, sel) {
        Entry::Builtin(n) => draw_builtin(&p.with_clip_rect(big), big, n, a),
        Entry::File(n) => {
            p.image(app.user_pics[n].1.id(), big, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
        }
        Entry::Ra => {
            if let Some(t) = app.ra_avatar() {
                p.image(t, big, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
            } else {
                ra_placeholder(&p, big, a, u);
            }
        }
        Entry::Own => {
            p.rect_filled(big, 4.0 * u, Color32::from_white_alpha((25.0 * a) as u8));
            crate::scene::icon_pub(&p, crate::Icon::Plus, big.center(), side * 0.22, Color32::WHITE.gamma_multiply(a));
        }
    }
    {
        p.rect_stroke(big, 4.0 * u, Stroke::new(1.5 * u, Color32::from_white_alpha((120.0 * a) as u8)), StrokeKind::Inside);
    }
    let mut y = big.bottom() + 18.0 * u;
    text_shadow(&p, pos2(tx, y + 12.0 * u), Align2::LEFT_CENTER, &title, 24.0 * u, Color32::WHITE.gamma_multiply(a));
    y += 34.0 * u;
    let g = p.layout(label, FontId::proportional(17.0 * u), Color32::from_rgb(200, 210, 220).gamma_multiply(a), f.panel.right() - tx - 30.0 * u);
    p.galley(pos2(tx, y), g, Color32::WHITE);
    let g = p.layout(
        "Tip: put your own square pictures (PNG/JPG) in theme\\gamerpics and they show up here.".to_string(),
        FontId::proportional(14.0 * u),
        Color32::from_white_alpha((150.0 * a) as u8),
        f.panel.right() - tx - 30.0 * u,
    );
    p.galley(pos2(tx, f.panel.bottom() - 74.0 * u), g, Color32::WHITE);
    hint_bar(&p, &[("A", "Select", BTN_A), ("B", "Back", BTN_B)], 100.0 * u, 650.0 * u, u, a);
}

// ---------------------------------------------------------------- crop editor

pub struct Cropper {
    pub img: image::RgbaImage,
    pub tex: egui::TextureHandle,
    /// 1.0 = the picture's short side exactly fills the frame.
    pub zoom: f32,
    /// Frame centre in picture coordinates (0..1).
    pub cx: f32,
    pub cy: f32,
    pub opened: Instant,
    drag: Option<(Pos2, f32, f32)>,
}

impl Cropper {
    fn clamp(&mut self) {
        self.zoom = self.zoom.clamp(1.0, 8.0);
        let (w, h) = (self.img.width() as f32, self.img.height() as f32);
        let side = w.min(h) / self.zoom;
        let hx = side / w * 0.5;
        let hy = side / h * 0.5;
        self.cx = self.cx.clamp(hx, 1.0 - hx);
        self.cy = self.cy.clamp(hy, 1.0 - hy);
    }
}

impl App {
    pub fn cropper_input(&mut self, p: Pad) {
        let Some(c) = self.cropper.as_mut() else { return };
        let step = 0.04 / c.zoom;
        match p {
            Pad::Left => c.cx -= step,
            Pad::Right => c.cx += step,
            Pad::Up => c.cy -= step,
            Pad::Down => c.cy += step,
            Pad::Rb | Pad::Y => c.zoom *= 1.15,
            Pad::Lb | Pad::X => c.zoom /= 1.15,
            Pad::B => {
                self.cropper = None;
                self.sounds.play("back");
                return;
            }
            Pad::A => {
                self.save_crop();
                return;
            }
            _ => {}
        }
        c.clamp();
    }

    fn save_crop(&mut self) {
        let Some(c) = self.cropper.take() else { return };
        let (w, h) = (c.img.width() as f32, c.img.height() as f32);
        let side = (w.min(h) / c.zoom).max(1.0);
        let x = (c.cx * w - side / 2.0).clamp(0.0, w - side) as u32;
        let y = (c.cy * h - side / 2.0).clamp(0.0, h - side) as u32;
        let cropped = image::imageops::crop_imm(&c.img, x, y, side as u32, side as u32).to_image();
        let out = image::imageops::resize(&cropped, 256, 256, image::imageops::FilterType::Lanczos3);
        let path = theme_dir().join("profile.png");
        let _ = std::fs::create_dir_all(theme_dir());
        match out.save(&path) {
            Ok(()) => {
                self.cfg.gamerpic = "custom".into();
                self.cfg.save();
                self.reload_custom_pic();
                self.sounds.play("select");
                self.toast("Gamerpic changed");
            }
            Err(e) => self.toast(format!("Couldn't save the picture: {e}")),
        }
    }
}

pub fn draw_cropper(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(c) = app.cropper.as_ref() else { return };
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let a = (c.opened.elapsed().as_secs_f32() / 0.22).min(1.0);
    let f = crate::menu::frame360(&p, app, screen, "Fit Your Picture", Some("Profile"), a, true);
    let u = f.u;
    let c = app.cropper.as_mut().unwrap();
    // Crop frame on the left, previews and instructions on the right.
    let s = 380.0 * u;
    let frame = Rect::from_center_size(pos2(f.panel.left() + 40.0 * u + s * 0.5, f.panel.center().y - 6.0 * u), vec2(s, s));
    let area = Rect::from_min_max(pos2(f.panel.left() + 6.0 * u, f.panel.top() + 6.0 * u), pos2(frame.right() + 34.0 * u, f.panel.bottom() - 6.0 * u));

    // Mouse: drag to move, wheel to zoom.
    let resp = ui.interact(area, egui::Id::unique("crop_area"), egui::Sense::drag());
    let (w, h) = (c.img.width() as f32, c.img.height() as f32);
    let scale = s * c.zoom / w.min(h); // screen px per picture px
    if resp.drag_started() {
        if let Some(pos) = resp.interact_pointer_pos() {
            c.drag = Some((pos, c.cx, c.cy));
        }
    }
    if let (Some((start, cx0, cy0)), Some(pos)) = (c.drag, resp.interact_pointer_pos()) {
        if resp.dragged() {
            let d = pos - start;
            c.cx = cx0 - d.x / (scale * w);
            c.cy = cy0 - d.y / (scale * h);
        }
    }
    if resp.drag_stopped() {
        c.drag = None;
    }
    if resp.hovered() {
        let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            c.zoom *= (1.0 + scroll * 0.002).max(0.5);
        }
    }
    c.clamp();

    let img_rect = Rect::from_min_size(pos2(frame.center().x - c.cx * w * scale, frame.center().y - c.cy * h * scale), vec2(w * scale, h * scale));
    let clip = p.with_clip_rect(area);
    clip.image(c.tex.id(), img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE.gamma_multiply(a));
    // Dim everything outside the frame.
    let dim = Color32::from_black_alpha((165.0 * a) as u8);
    clip.rect_filled(Rect::from_min_max(area.min, pos2(area.right(), frame.top())), 0.0, dim);
    clip.rect_filled(Rect::from_min_max(pos2(area.left(), frame.bottom()), area.max), 0.0, dim);
    clip.rect_filled(Rect::from_min_max(pos2(area.left(), frame.top()), pos2(frame.left(), frame.bottom())), 0.0, dim);
    clip.rect_filled(Rect::from_min_max(pos2(frame.right(), frame.top()), pos2(area.right(), frame.bottom())), 0.0, dim);
    p.rect_stroke(frame, 4.0 * u, Stroke::new(2.0 * u, Color32::WHITE.gamma_multiply(a)), StrokeKind::Outside);
    // Rule-of-thirds guides while dragging.
    if c.drag.is_some() {
        for k in 1..3 {
            let t = k as f32 / 3.0;
            p.line_segment([pos2(frame.left() + s * t, frame.top()), pos2(frame.left() + s * t, frame.bottom())], Stroke::new(1.0, Color32::from_white_alpha(70)));
            p.line_segment([pos2(frame.left(), frame.top() + s * t), pos2(frame.right(), frame.top() + s * t)], Stroke::new(1.0, Color32::from_white_alpha(70)));
        }
    }

    // Live previews at the sizes the dashboard uses.
    let side = w.min(h) / c.zoom;
    let uv = Rect::from_min_size(pos2((c.cx * w - side / 2.0) / w, (c.cy * h - side / 2.0) / h), vec2(side / w, side / h));
    let rx = frame.right() + 60.0 * u;
    text_shadow(&p, pos2(rx, f.panel.top() + 36.0 * u), Align2::LEFT_CENTER, "Preview", 20.0 * u, Color32::WHITE.gamma_multiply(a));
    let big = Rect::from_min_size(pos2(rx, f.panel.top() + 60.0 * u), vec2(128.0 * u, 128.0 * u));
    let small = Rect::from_min_size(pos2(big.right() + 24.0 * u, big.top()), vec2(64.0 * u, 64.0 * u));
    for r in [big, small] {
        p.image(c.tex.id(), r, uv, Color32::WHITE.gamma_multiply(a));
        p.rect_stroke(r, 4.0 * u, Stroke::new(1.5 * u, Color32::from_white_alpha((140.0 * a) as u8)), StrokeKind::Inside);
    }
    let help = "Move the picture with the stick, d-pad or arrow keys (or drag it with the mouse).\n\nZoom in with RB or Y, zoom out with LB or X (or use the mouse wheel).\n\nPress A to save it as your gamerpic.";
    let g = p.layout(help.to_string(), FontId::proportional(18.0 * u), Color32::from_rgb(214, 222, 230).gamma_multiply(a), f.panel.right() - rx - 30.0 * u);
    p.galley(pos2(rx, big.bottom() + 28.0 * u), g, Color32::WHITE);
    // Zoom bar.
    let bar = Rect::from_min_size(pos2(rx, f.panel.bottom() - 46.0 * u), vec2(f.panel.right() - rx - 40.0 * u, 6.0 * u));
    p.rect_filled(bar, 3.0 * u, Color32::from_black_alpha((90.0 * a) as u8));
    let t = (c.zoom.ln() / 8f32.ln()).clamp(0.0, 1.0);
    p.circle_filled(pos2(bar.left() + bar.width() * t, bar.center().y), 9.0 * u, Color32::from_rgb(150, 206, 62).gamma_multiply(a));
    p.text(pos2(bar.left(), bar.top() - 16.0 * u), Align2::LEFT_CENTER, format!("Zoom {:.1}x", c.zoom), FontId::proportional(15.0 * u), Color32::from_white_alpha((190.0 * a) as u8));
    hint_bar(
        &p,
        &[("A", "Save", BTN_A), ("B", "Cancel", BTN_B), ("LB", "Zoom Out", BTN_GREY), ("RB", "Zoom In", BTN_GREY)],
        100.0 * u,
        650.0 * u,
        u,
        a,
    );
}

/// Draws the current gamerpic into `r`.
pub fn draw_current(app: &App, p: &egui::Painter, r: Rect, a: f32) {
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    match app.cfg.gamerpic.as_str() {
        "custom" if app.custom_pic().is_some() => {
            p.image(app.custom_pic().unwrap(), r, uv, Color32::WHITE.gamma_multiply(a));
        }
        "ra" if app.ra_avatar().is_some() => {
            p.image(app.ra_avatar().unwrap(), r, uv, Color32::WHITE.gamma_multiply(a));
        }
        s if s.starts_with("file:") && app.user_pics.iter().any(|(k, _)| split_key(k).0 == &s[5..]) => {
            let t = app.user_pics.iter().find(|(k, _)| split_key(k).0 == &s[5..]).unwrap().1.id();
            p.image(t, r, uv, Color32::WHITE.gamma_multiply(a));
        }
        s => {
            let n = s.strip_prefix("builtin:").and_then(|n| n.parse().ok()).unwrap_or(0);
            draw_builtin(&p.with_clip_rect(r), r, n, a);
        }
    }
}
