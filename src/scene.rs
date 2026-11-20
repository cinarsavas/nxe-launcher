//! Dashboard rendering in the NXE layout: bright sky over a curved silver
//! floor, channel names stacked top-left, one row of tiles that shrink into the
//! distance, profile in the top-right and button hints bottom-left.

use crate::{App, Icon};
use eframe::egui::{self, pos2, vec2, Align2, Color32, CornerRadius, FontId, Mesh, Pos2, Rect, Shape, Stroke, TextureId};
use std::collections::HashMap;

// ------------------------------------------------------------------ theme

#[derive(Clone, Copy)]
pub struct Theme {
    pub sky_top: Color32,
    pub sky_mid: Color32,
    pub haze: Color32,
    pub glow: Color32,
    pub tile_hi: Color32,
    pub tile_lo: Color32,
}

pub const THEMES: [&str; 5] = ["green", "blue", "orange", "purple", "black"];

pub fn theme(name: &str) -> Theme {
    let c = Color32::from_rgb;
    match name {
        "blue" => Theme {
            sky_top: c(18, 74, 142),
            sky_mid: c(86, 156, 218),
            haze: c(228, 238, 247),
            glow: c(150, 215, 255),
            tile_hi: c(96, 176, 240),
            tile_lo: c(22, 82, 168),
        },
        "orange" => Theme {
            sky_top: c(160, 64, 8),
            sky_mid: c(236, 146, 56),
            haze: c(250, 238, 224),
            glow: c(255, 214, 120),
            tile_hi: c(250, 164, 64),
            tile_lo: c(178, 66, 8),
        },
        "purple" => Theme {
            sky_top: c(70, 26, 112),
            sky_mid: c(152, 104, 206),
            haze: c(240, 232, 248),
            glow: c(228, 176, 255),
            tile_hi: c(170, 112, 222),
            tile_lo: c(78, 30, 140),
        },
        "black" => Theme {
            sky_top: c(16, 18, 22),
            sky_mid: c(54, 60, 68),
            haze: c(138, 144, 150),
            glow: c(110, 120, 132),
            tile_hi: c(104, 112, 122),
            tile_lo: c(34, 38, 44),
        },
        _ => Theme {
            sky_top: c(72, 132, 14),
            sky_mid: c(156, 204, 62),
            haze: c(236, 242, 224),
            glow: c(244, 236, 96),
            tile_hi: c(186, 226, 58),
            tile_lo: c(64, 134, 8),
        },
    }
}

fn mul(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a.clamp(0.0, 1.0))
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
}

fn lighten(c: Color32, k: f32) -> Color32 {
    let l = |x: u8| (x as f32 + (255.0 - x as f32) * k) as u8;
    Color32::from_rgb(l(c.r()), l(c.g()), l(c.b()))
}

fn darken(c: Color32, k: f32) -> Color32 {
    let l = |x: u8| (x as f32 * (1.0 - k)) as u8;
    Color32::from_rgb(l(c.r()), l(c.g()), l(c.b()))
}

pub fn text_shadow(p: &egui::Painter, pos: Pos2, align: Align2, s: &str, size: f32, col: Color32) {
    let shadow = Color32::from_black_alpha((col.a() as f32 * 0.45) as u8);
    p.text(pos + vec2(size * 0.05, size * 0.07), align, s, FontId::proportional(size), shadow);
    p.text(pos, align, s, FontId::proportional(size), col);
}

/// Vertical gradient rectangle.
fn grad_rect(r: Rect, top: Color32, bottom: Color32) -> Shape {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.left_bottom(), bottom);
    m.colored_vertex(r.right_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    Shape::mesh(m)
}

/// Textured rect with per-edge colours (used for reflections).
fn tex_rect(tex: TextureId, r: Rect, uv: Rect, top: Color32, bottom: Color32) -> Shape {
    let mut m = Mesh::with_texture(tex);
    let v = |p: Pos2, uv: Pos2, c: Color32| egui::epaint::Vertex { pos: p, uv, color: c };
    m.vertices.push(v(r.left_top(), uv.left_top(), top));
    m.vertices.push(v(r.right_top(), uv.right_top(), top));
    m.vertices.push(v(r.left_bottom(), uv.left_bottom(), bottom));
    m.vertices.push(v(r.right_bottom(), uv.right_bottom(), bottom));
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    Shape::mesh(m)
}

fn radial(c: Pos2, r: Pos2, inner: Color32) -> Shape {
    let mut m = Mesh::default();
    m.colored_vertex(c, inner);
    let n = 48;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(pos2(c.x + a.cos() * r.x, c.y + a.sin() * r.y), Color32::TRANSPARENT);
        if i > 0 {
            m.add_triangle(0, i as u32, i as u32 + 1);
        }
    }
    Shape::mesh(m)
}

/// Sky, bokeh, the curved floor and its ripples. Shared by every screen.
pub fn lerp_theme(a: &Theme, b: &Theme, t: f32) -> Theme {
    Theme {
        sky_top: lerp_color(a.sky_top, b.sky_top, t),
        sky_mid: lerp_color(a.sky_mid, b.sky_mid, t),
        haze: lerp_color(a.haze, b.haze, t),
        glow: lerp_color(a.glow, b.glow, t),
        tile_hi: lerp_color(a.tile_hi, b.tile_hi, t),
        tile_lo: lerp_color(a.tile_lo, b.tile_lo, t),
    }
}

/// Ease-out cubic of `(t - start) / dur`, clamped to 0..1.
pub fn seg(t: f32, start: f32, dur: f32) -> f32 {
    let x = ((t - start) / dur).clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

/// `rise` 0..1 lifts the floor into place (used by the intro).
pub fn background(p: &egui::Painter, screen: Rect, t: f32, th: &Theme, bg_tex: Option<TextureId>, rise: f32) {
    let (w, h) = (screen.width(), screen.height());
    if let Some(tex) = bg_tex {
        p.image(tex, screen, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        return;
    }
    let horizon = h * 0.565 + (1.0 - rise) * h * 0.45;
    // Sky.
    p.add(grad_rect(Rect::from_min_max(pos2(0.0, 0.0), pos2(w, h * 0.32)), th.sky_top, th.sky_mid));
    p.add(grad_rect(Rect::from_min_max(pos2(0.0, h * 0.32 - 0.5), pos2(w, horizon + 4.0)), th.sky_mid, th.haze));
    p.add(grad_rect(Rect::from_min_max(pos2(0.0, horizon + 3.0), pos2(w, h)), th.haze, th.haze));
    // Warm glow on the right of the horizon and a soft white bloom in the middle.
    p.add(radial(pos2(w * 1.02, horizon * 0.98), pos2(w * 0.36, h * 0.30), mul(th.glow, 0.95)));
    p.add(radial(pos2(w * 0.5, horizon), pos2(w * 0.55, h * 0.16), Color32::from_white_alpha(120)));
    // Bokeh rings drifting slowly.
    let mut seed = 7u32;
    let mut rnd = || {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (seed >> 8) as f32 / 16_777_216.0
    };
    for i in 0..26 {
        let x = (rnd() * 1.2 * w + t * (4.0 + rnd() * 6.0)) % (w * 1.2) - w * 0.1;
        let y = rnd() * horizon * 0.95;
        let r = h * (0.006 + rnd() * 0.035);
        let a = (6.0 + rnd() * 16.0) as u8;
        if i % 3 == 0 {
            p.circle_stroke(pos2(x, y), r, Stroke::new(r * 0.18, Color32::from_white_alpha(a)));
            p.circle_stroke(pos2(x, y), r * 0.55, Stroke::new(r * 0.12, Color32::from_white_alpha(a / 2)));
        } else {
            p.circle_filled(pos2(x, y), r, Color32::from_white_alpha(a));
        }
    }
    // Floor: a huge sphere seen from just above.
    let r_big = w * 1.7;
    let cx = w * 0.42;
    let cy = horizon + r_big;
    let rim = Color32::from_rgb(214, 218, 222);
    let mid = Color32::from_rgb(150, 156, 162);
    let low = Color32::from_rgb(78, 84, 90);
    let mut m = Mesh::default();
    let cols = 64;
    for i in 0..=cols {
        let x = -w * 0.05 + w * 1.1 * i as f32 / cols as f32;
        let dx = x - cx;
        let y = cy - (r_big * r_big - dx * dx).max(0.0).sqrt();
        m.colored_vertex(pos2(x, y), rim);
        m.colored_vertex(pos2(x, y + h * 0.10), mid);
        m.colored_vertex(pos2(x, h), low);
        if i > 0 {
            let b = ((i - 1) * 3) as u32;
            let c = (i * 3) as u32;
            m.add_triangle(b, c, b + 1);
            m.add_triangle(c, c + 1, b + 1);
            m.add_triangle(b + 1, c + 1, b + 2);
            m.add_triangle(c + 1, c + 2, b + 2);
        }
    }
    p.add(Shape::mesh(m));
    // Bright rim line where floor meets sky.
    let mut rim_pts = Vec::new();
    for i in 0..=cols {
        let x = -w * 0.05 + w * 1.1 * i as f32 / cols as f32;
        let dx = x - cx;
        rim_pts.push(pos2(x, cy - (r_big * r_big - dx * dx).max(0.0).sqrt()));
    }
    p.add(Shape::line(rim_pts, Stroke::new(h * 0.003, Color32::from_white_alpha(140))));
    // Ripples bottom-right.
    let rc = pos2(w * 0.905, h * 0.885);
    for k in 1..7 {
        let rr = h * 0.045 * k as f32 + (t * 6.0) % (h * 0.045);
        let a = (40.0 * (1.0 - k as f32 / 7.0)) as u8;
        let pts: Vec<Pos2> = (0..=60)
            .map(|i| {
                let ang = i as f32 / 60.0 * std::f32::consts::TAU;
                pos2(rc.x + ang.cos() * rr * 1.6, rc.y + ang.sin() * rr * 0.42)
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(1.2, Color32::from_black_alpha(a))));
    }
}

// ------------------------------------------------------------------ dashboard

/// The sky/floor behind everything. Drawn on its own layer so pages can zoom over it.
pub fn draw_background(app: &App, ui: &egui::Ui, ctx: &egui::Context) {
    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let it = app.intro_t().unwrap_or(10.0);
    background(&p, screen, app.time(), &app.theme_now(), app.background(), seg(it, 0.05, 0.9));
}

pub fn draw(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let n = app.channel_count();
    let chan_anim = app.chan_anim();
    // Pre-load the covers we may show this frame.
    let mut covers: HashMap<usize, (TextureId, f32, Color32)> = HashMap::new();
    let wanted: Vec<usize> = app
        .visible_channels(chan_anim)
        .into_iter()
        .flat_map(|(ci, _)| {
            let c = &app.channels_ref()[ci];
            let lo = c.sel.saturating_sub(2);
            c.tiles.iter().skip(lo).take(9).filter_map(|t| t.art.or(t.game)).collect::<Vec<_>>()
        })
        .collect();
    for gi in wanted {
        if let Some(t) = app.cover(ctx, gi) {
            covers.insert(gi, t);
        }
    }

    let screen = ctx.content_rect();
    let p = ui.painter_at(screen);
    let (w, h) = (screen.width(), screen.height());
    let u = (h / 720.0).min(w / 1280.0);
    let t = app.time();
    let th = app.theme_now();
    let intro = app.intro_t();
    let it = intro.unwrap_or(10.0);
    let _ = (t, &th, it);

    let x0 = 96.0 * u;
    let tile_w = 420.0 * u;
    let tile_h = 320.0 * u;
    let tile_top = 248.0 * u;

    // ---------- channel names, top-left (current at the bottom, following ones stacked above)
    let ys = [0.0, -44.0, -77.0, -103.0, -128.0, -150.0];
    let sizes = [36.0, 27.0, 22.0, 19.0, 17.0, 15.0];
    let alphas = [1.0, 0.72, 0.5, 0.32, 0.18, 0.0];
    let cur_y = 203.0 * u;
    for ci in 0..n {
        let mut o = (ci as f32 - chan_anim).rem_euclid(n as f32);
        if o > n as f32 - 1.0 {
            o -= n as f32; // leaving downwards
        }
        let (y, size, a) = if o < 0.0 {
            (cur_y - o * 40.0 * u, 36.0, (1.0 + o * 2.0).max(0.0))
        } else {
            let i = (o.floor() as usize).min(4);
            let f = o - i as f32;
            let lerp = |arr: &[f32; 6]| arr[i] + (arr[i + 1] - arr[i]) * f;
            (cur_y + lerp(&ys) * u, lerp(&sizes), lerp(&alphas))
        };
        if a <= 0.01 {
            continue;
        }
        let name = app.channels_ref()[ci].name.clone();
        let ie = seg(it, 0.35 + o.max(0.0) * 0.07, 0.5);
        let a = a * ie;
        let x0 = x0 - (1.0 - ie) * 70.0 * u;
        let col = mul(Color32::WHITE, a * if o.abs() < 0.5 { 1.0 } else { 0.9 });
        text_shadow(&p, pos2(x0, y), Align2::LEFT_CENTER, &name, size * u, col);
        if ci == 0 {
            // Home marker.
            let s = size * u * 0.17;
            p.rect_filled(Rect::from_center_size(pos2(x0 - s * 2.4, y + size * u * 0.02), vec2(s, s)), 0.0, col);
        }
    }

    // ---------- tiles (current channel, plus the one sliding in/out)
    for (ci, o) in app.visible_channels(chan_anim) {
        let row_a = (1.0 - o.abs()).clamp(0.0, 1.0).powf(1.4);
        let dy = -o * 260.0 * u;
        let c = &app.channels_ref()[ci];
        let k = 0.36f32;
        let norm = k / (1.0 + k).ln();
        let mut order: Vec<usize> = (0..c.tiles.len()).filter(|&i| (-1.2..8.0).contains(&(i as f32 - c.anim))).collect();
        order.sort_by(|a, b| b.cmp(a));
        for i in order {
            let tile = &c.tiles[i];
            let tt = i as f32 - c.anim;
            let (left, s, mut a) = if tt >= 0.0 {
                let s = 1.0 / (1.0 + k * tt);
                (x0 + tile_w * norm * (1.0 + k * tt).ln() / k, s, (1.0 - (tt - 5.0) / 2.0).clamp(0.0, 1.0))
            } else {
                (x0 + tt * tile_w * 1.08, 1.0, (1.0 + tt * 1.6).clamp(0.0, 1.0))
            };
            a *= row_a;
            if a <= 0.01 {
                continue;
            }
            let focused = i == c.sel && ci == app.chan_index();
            // Selection "pop", launch zoom and the slide-in when coming back from a game.
            let mut grow = 1.0;
            let mut left = left;
            if focused {
                let t = (app.sel_age() / 0.28).min(1.0);
                grow += 0.035 * (t * std::f32::consts::PI).sin() * (1.0 - t);
                if let Some((el, _)) = app.launching() {
                    let e = (el / 0.6).min(1.0);
                    grow += 0.10 * (1.0 - (1.0 - e).powi(3));
                }
            }
            if let Some(el) = app.returned() {
                let e = (el / 0.75).min(1.0);
                let ease = 1.0 - (1.0 - e).powi(4);
                left += (1.0 - ease) * 140.0 * u * (1.0 + tt.max(0.0) * 0.6);
                a *= ease.max(0.2);
            }
            if intro.is_some() {
                let ie = seg(it, 0.5 + tt.max(0.0) * 0.08, 0.6);
                left += (1.0 - ie) * w * 0.45;
                a *= ie;
                grow *= 0.92 + 0.08 * ie;
            }
            let tw = tile_w * s * grow;
            let thh = tile_h * s * grow;
            let cy = tile_top + tile_h * 0.5 - (1.0 - s) * tile_h * 0.06 + dy;
            let left = left - (grow - 1.0) * tile_w * s * 0.5;
            let r = Rect::from_min_size(pos2(left, cy - thh * 0.5), vec2(tw, thh));
            let dim = if focused { 1.0 } else { 0.9 };
            let art = tile.art.or(tile.game).and_then(|g| covers.get(&g).copied());
            let (hi, lo) = match art {
                Some((_, _, avg)) => (lighten(avg, 0.22), darken(avg, 0.55)),
                None => (tile.accent.unwrap_or(th.tile_hi), tile.accent.map(|c| darken(c, 0.5)).unwrap_or(th.tile_lo)),
            };
            let hi = mul(lerp_color(hi, Color32::BLACK, 1.0 - dim), a);
            let lo = mul(lerp_color(lo, Color32::BLACK, 1.0 - dim), a);

            // Reflection on the floor.
            let refl = Rect::from_min_size(pos2(r.left(), r.bottom() + 2.0 * u), vec2(tw, thh * 0.28));
            p.add(grad_rect(refl, mul(lo, 0.30), Color32::TRANSPARENT));

            // Body.
            p.add(grad_rect(r, hi, lo));
            // Decorative rings (like the original's pattern), clipped to the tile.
            {
                let clip = p.with_clip_rect(r);
                let mut seed = (i as u32 + 1).wrapping_mul(2654435761) | 1;
                let mut rnd = || {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    (seed % 10_000) as f32 / 10_000.0
                };
                for _ in 0..5 {
                    let c0 = pos2(r.left() + rnd() * tw, r.top() + rnd() * thh);
                    let rr = thh * (0.04 + rnd() * 0.12);
                    clip.circle_stroke(c0, rr, Stroke::new(rr * 0.14, mul(Color32::WHITE, a * 0.045)));
                    clip.circle_filled(c0, rr * 0.35, mul(Color32::WHITE, a * 0.03));
                }
            }
            // Gloss across the top.
            p.add(grad_rect(Rect::from_min_max(r.min, pos2(r.right(), r.top() + thh * 0.45)), mul(Color32::WHITE, a * 0.16), Color32::TRANSPARENT));

            if let Some((tex, aspect, _)) = art {
                let pad = 14.0 * u * s;
                let bw = tw - pad * 2.0;
                let bh = thh * 0.70;
                let (cw, ch) = if aspect > bw / bh { (bw, bw / aspect) } else { (bh * aspect, bh) };
                let ar = Rect::from_center_size(pos2(r.center().x, r.top() + pad + bh * 0.5), vec2(cw, ch));
                p.rect_filled(ar.expand(3.0 * u * s).translate(vec2(0.0, 4.0 * u * s)), 2.0, mul(Color32::from_black_alpha(80), a));
                p.image(tex, ar, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), mul(Color32::WHITE, a * dim));
                // Art reflection.
                let rr = Rect::from_min_size(pos2(ar.left(), r.bottom() + 2.0 * u + (r.bottom() - ar.bottom())), vec2(cw, ch * 0.25));
                if rr.top() < h {
                    p.add(tex_rect(tex, rr, Rect::from_min_max(pos2(0.0, 1.0), pos2(1.0, 0.75)), mul(Color32::WHITE, a * 0.16), Color32::TRANSPARENT));
                }
            } else {
                icon(&p, tile.glyph, pos2(r.center().x, r.top() + thh * 0.42), thh * 0.2, mul(Color32::from_white_alpha(235), a));
            }

            // Labels, bottom-left.
            let lx = r.left() + 18.0 * u * s;
            let title_size = 24.0 * u * s.max(0.55);
            let max_w = tw - 30.0 * u * s;
            let title = fit(&p, &tile.title, &FontId::proportional(title_size), max_w);
            if s > 0.45 {
                text_shadow(&p, pos2(lx, r.bottom() - 46.0 * u * s), Align2::LEFT_CENTER, &title, title_size, mul(Color32::WHITE, a));
                let sub = fit(&p, &tile.subtitle, &FontId::proportional(15.0 * u * s.max(0.7)), max_w);
                text_shadow(&p, pos2(lx, r.bottom() - 22.0 * u * s), Align2::LEFT_CENTER, &sub, 15.0 * u * s.max(0.7), mul(Color32::from_white_alpha(215), a));
            }
        }
        // "1 of 8" + achievements under the focused tile.
        if c.tiles.len() > 1 || app.focused_progress().is_some() {
            let y = tile_top + tile_h + 20.0 * u + dy;
            let ca = row_a * seg(it, 0.9, 0.4);
            if c.tiles.len() > 1 {
                text_shadow(&p, pos2(x0, y), Align2::LEFT_CENTER, &format!("{} of {}", c.sel + 1, c.tiles.len()), 17.0 * u, mul(Color32::from_white_alpha(225), ca));
            }
            if ci == app.chan_index() {
                if let Some((earned, total)) = app.focused_progress() {
                    let bx = x0 + 120.0 * u;
                    let bw = 200.0 * u;
                    icon(&p, Icon::Trophy, pos2(bx, y), 9.0 * u, mul(Color32::WHITE, ca));
                    let bar = Rect::from_min_size(pos2(bx + 18.0 * u, y - 4.0 * u), vec2(bw, 8.0 * u));
                    p.rect_filled(bar, 4.0 * u, mul(Color32::from_black_alpha(70), ca));
                    let f = if total > 0 { earned as f32 / total as f32 } else { 0.0 };
                    p.rect_filled(Rect::from_min_size(bar.min, vec2(bw * f, 8.0 * u)), 4.0 * u, mul(th.tile_hi, ca));
                    text_shadow(&p, pos2(bar.right() + 12.0 * u, y), Align2::LEFT_CENTER, &format!("{earned} / {total}"), 16.0 * u, mul(Color32::WHITE, ca));
                }
            }
        }
    }

    // ---------- profile, top-right
    {
        let pe = seg(it, 0.75, 0.5);
        if pe > 0.01 {
            // Slides down from above.
            draw_profile_at(app, &p, w, u, -(1.0 - pe) * 90.0 * u, pe);
        }
    }

    // ---------- hints, bottom-left
    if !app.menu_open() && !app.installing.as_ref().map(|i| i.visible).unwrap_or(false) {
        let mut hints: Vec<(&str, &str, Color32)> = vec![("A", if app.focused_game().is_some() { "Play" } else { "Select" }, BTN_A)];
        if app.focused_game().is_some() {
            if app.focused_progress().is_some() {
                hints.push(("X", "Achievements", BTN_X));
            }
            hints.push(("Y", "Change Cover", BTN_Y));
        }
        hints.push(("START", "Settings", BTN_GREY));
        hint_bar(&p, &hints, 100.0 * u, 650.0 * u, u, seg(it, 1.0, 0.4));
    }

    // ---------- toast (notification pill, bottom-centre like a sign-in popup)
    if let Some((s, age)) = app.toast_text() {
        let a = ((age / 0.2).min(1.0) * (1.0 - (age - 2.6).max(0.0) / 0.4)).clamp(0.0, 1.0);
        let g = p.layout_no_wrap(s.to_string(), FontId::proportional(19.0 * u), mul(Color32::WHITE, a));
        let rise = (1.0 - (age / 0.25).min(1.0)).powi(3) * 40.0 * u;
        let r = Rect::from_center_size(pos2(w * 0.5, 620.0 * u + rise), g.size() + vec2(70.0 * u, 26.0 * u));
        p.rect_filled(r, CornerRadius::same((r.height() * 0.5) as u8), mul(Color32::from_rgb(40, 44, 50), a * 0.92));
        p.rect_stroke(r, CornerRadius::same((r.height() * 0.5) as u8), Stroke::new(1.5 * u, mul(Color32::from_white_alpha(70), a)), egui::StrokeKind::Inside);
        let dot = pos2(r.left() + 24.0 * u, r.center().y);
        p.circle_filled(dot, 9.0 * u, mul(th.tile_hi, a));
        p.galley(pos2(dot.x + 18.0 * u, r.center().y - g.size().y * 0.5), g, Color32::WHITE);
    }

    // ---------- built-in core download
    if let Some(ins) = app.installing.as_mut().filter(|i| i.visible) {
        let el = ins.started.elapsed().as_secs_f32();
        let a = (el / 0.2).min(1.0);
        ins.shown += (ins.frac - ins.shown) * 0.15;
        p.rect_filled(screen, 0.0, Color32::from_black_alpha((150.0 * a) as u8));
        let slide = (1.0 - a).powi(3) * 30.0 * u;
        let panel = Rect::from_center_size(pos2(w * 0.5, h * 0.5 + slide), vec2(620.0 * u, 190.0 * u));
        p.rect_filled(panel, 6.0 * u, Color32::from_rgba_unmultiplied(66, 82, 100, (240.0 * a) as u8));
        p.rect_stroke(panel, 6.0 * u, Stroke::new(1.0, Color32::from_white_alpha(30)), egui::StrokeKind::Inside);
        let title = format!("Getting {} ready", ins.system.channel());
        text_shadow(&p, panel.min + vec2(28.0 * u, 36.0 * u), Align2::LEFT_CENTER, &title, 26.0 * u, mul(Color32::WHITE, a));
        let (line, col) = match &ins.error {
            Some(e) => (format!("Download failed: {e}"), Color32::from_rgb(255, 170, 120)),
            None => (format!("{}…", ins.stage), Color32::from_rgb(210, 220, 230)),
        };
        let line = fit(&p, &line, &FontId::proportional(18.0 * u), panel.width() - 56.0 * u);
        p.text(panel.min + vec2(28.0 * u, 76.0 * u), Align2::LEFT_CENTER, line, FontId::proportional(18.0 * u), mul(col, a));
        let bar = Rect::from_min_size(panel.min + vec2(28.0 * u, 104.0 * u), vec2(panel.width() - 56.0 * u, 14.0 * u));
        p.rect_filled(bar, 7.0 * u, mul(Color32::from_black_alpha(90), a));
        let fill = Rect::from_min_size(bar.min, vec2(bar.width() * ins.shown.clamp(0.0, 1.0), bar.height()));
        p.add(grad_rect(fill, mul(Color32::from_rgb(160, 214, 70), a), mul(Color32::from_rgb(80, 150, 24), a)));
        // Moving sheen on the bar so it never looks frozen.
        let sx = bar.left() + ((t * 0.6) % 1.0) * bar.width();
        if sx < fill.right() {
            p.rect_filled(Rect::from_center_size(pos2(sx, bar.center().y), vec2(40.0 * u, bar.height())), 7.0 * u, Color32::from_white_alpha((40.0 * a) as u8));
        }
        p.text(
            pos2(bar.right(), bar.bottom() + 22.0 * u),
            Align2::RIGHT_CENTER,
            format!("{:.0}%", ins.shown * 100.0),
            FontId::proportional(16.0 * u),
            mul(Color32::from_white_alpha(200), a),
        );
        p.text(
            pos2(bar.left(), bar.bottom() + 22.0 * u),
            Align2::LEFT_CENTER,
            if ins.error.is_some() { "" } else { "Only needed once. The game starts right after." },
            FontId::proportional(15.0 * u),
            mul(Color32::from_white_alpha(170), a),
        );
        hint_bar(&p, &[("B", if ins.error.is_some() { "Close" } else { "Hide" }, BTN_B)], 100.0 * u, 650.0 * u, u, a);
    }

    // ---------- launch fade / playing screen / return fade
    let overlay = |a: f32, title: &str, sub: &str| {
        p.rect_filled(screen, 0.0, Color32::from_black_alpha((a * 255.0) as u8));
        text_shadow(&p, pos2(w * 0.5, h * 0.47), Align2::CENTER_CENTER, title, 36.0 * u, mul(Color32::WHITE, a));
        p.text(pos2(w * 0.5, h * 0.54), Align2::CENTER_CENTER, sub, FontId::proportional(19.0 * u), mul(Color32::from_white_alpha(190), a));
    };
    if let Some((el, name)) = app.launching() {
        overlay((el / 0.5).min(1.0), &name, "Starting…");
    } else if let Some(name) = app.playing() {
        overlay(0.92, name, "Playing. Press Guide or hold Back + Start to return");
    } else if let Some(el) = app.returned() {
        p.rect_filled(screen, 0.0, Color32::from_black_alpha(((1.0 - el / 0.45).clamp(0.0, 1.0).powi(2) * 255.0) as u8));
    }
    if intro.is_some() {
        let fade = 1.0 - seg(it, 0.0, 0.45);
        if fade > 0.0 {
            p.rect_filled(screen, 0.0, Color32::from_black_alpha((fade * 255.0) as u8));
        }
    }
}

pub const BTN_A: Color32 = Color32::from_rgb(96, 178, 40);
pub const BTN_B: Color32 = Color32::from_rgb(210, 40, 40);
pub const BTN_X: Color32 = Color32::from_rgb(40, 110, 220);
pub const BTN_Y: Color32 = Color32::from_rgb(232, 180, 20);
pub const BTN_GREY: Color32 = Color32::from_rgb(90, 96, 104);

/// Controller-style button hints, left-aligned at (x, y).
pub fn hint_bar(p: &egui::Painter, hints: &[(&str, &str, Color32)], x: f32, y: f32, u: f32, alpha: f32) {
    let mut x = x;
    for (letter, label, col) in hints {
        let col = col.gamma_multiply(alpha);
        if letter.chars().count() > 1 {
            let g = p.layout_no_wrap(letter.to_string(), FontId::proportional(10.0 * u), Color32::WHITE.gamma_multiply(alpha));
            let r = Rect::from_min_size(pos2(x, y - 9.0 * u), vec2(g.size().x + 14.0 * u, 18.0 * u));
            p.rect_filled(r, CornerRadius::same((9.0 * u) as u8), col);
            p.galley(r.center() - g.size() * 0.5, g, Color32::WHITE);
            x = r.right() + 6.0 * u;
        } else {
            let c = pos2(x + 11.0 * u, y);
            p.circle_filled(c + vec2(0.0, 1.5 * u), 11.0 * u, Color32::from_black_alpha((80.0 * alpha) as u8));
            p.circle_filled(c, 11.0 * u, col);
            p.circle_filled(c - vec2(0.0, 4.0 * u), 6.0 * u, Color32::from_white_alpha((50.0 * alpha) as u8));
            p.text(c, Align2::CENTER_CENTER, *letter, FontId::proportional(13.0 * u), Color32::WHITE.gamma_multiply(alpha));
            x = c.x + 18.0 * u;
        }
        let g = p.layout_no_wrap(label.to_string(), FontId::proportional(18.0 * u), Color32::WHITE.gamma_multiply(alpha));
        let lw = g.size().x;
        p.galley(pos2(x + 1.0, y - g.size().y * 0.5 + 1.5), p.layout_no_wrap(label.to_string(), FontId::proportional(18.0 * u), Color32::from_black_alpha((90.0 * alpha) as u8)), Color32::BLACK);
        p.galley(pos2(x, y - g.size().y * 0.5), g, Color32::WHITE);
        x += lw + 24.0 * u;
    }
}

pub fn draw_profile_at(app: &App, p: &egui::Painter, w: f32, u: f32, dy: f32, a: f32) {
    let av = Rect::from_min_size(pos2(w - 160.0 * u, 64.0 * u + dy), vec2(64.0 * u, 64.0 * u));
    p.rect_filled(av.translate(vec2(0.0, 3.0 * u)), 4.0 * u, Color32::from_black_alpha((60.0 * a) as u8));
    crate::profile::draw_current(app, p, av, a);
    p.rect_stroke(av, 4.0 * u, Stroke::new(1.5 * u, Color32::from_white_alpha((120.0 * a) as u8)), egui::StrokeKind::Inside);
    match app.ra_points() {
        Some(points) => {
            text_shadow(p, pos2(av.left() - 10.0 * u, 79.0 * u + dy), Align2::RIGHT_CENTER, &app.cfg.gamertag, 25.0 * u, mul(Color32::WHITE, a));
            let coin = pos2(av.left() - 21.0 * u, 112.0 * u + dy);
            p.circle_filled(coin, 10.0 * u, Color32::from_white_alpha((235.0 * a) as u8));
            icon(p, Icon::Trophy, coin, 6.0 * u, mul(Color32::from_rgb(90, 96, 104), a));
            text_shadow(p, pos2(coin.x - 15.0 * u, coin.y), Align2::RIGHT_CENTER, &points.to_string(), 22.0 * u, mul(Color32::WHITE, a));
        }
        // No achievements account: just the name, centred on the picture.
        None => text_shadow(p, pos2(av.left() - 12.0 * u, av.center().y), Align2::RIGHT_CENTER, &app.cfg.gamertag, 25.0 * u, mul(Color32::WHITE, a)),
    }
}

/// Shortens text with "…" to fit max_w.
pub fn fit(p: &egui::Painter, s: &str, font: &FontId, max_w: f32) -> String {
    let w = |t: &str| p.layout_no_wrap(t.to_string(), font.clone(), Color32::WHITE).size().x;
    if w(s) <= max_w {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut n = chars.len();
    while n > 1 {
        n -= 1;
        let t: String = chars[..n].iter().collect::<String>() + "…";
        if w(&t) <= max_w {
            return t;
        }
    }
    "…".into()
}

pub fn icon_pub(p: &egui::Painter, kind: Icon, c: Pos2, r: f32, col: Color32) {
    icon(p, kind, c, r, col)
}

/// Simple vector icons (no font glyphs needed). `r` = radius.
fn icon(p: &egui::Painter, kind: Icon, c: Pos2, r: f32, col: Color32) {
    use egui::vec2;
    let st = Stroke::new((r * 0.16).max(1.0), col);
    match kind {
        Icon::Play => {
            let pts = vec![c + vec2(-r * 0.55, -r * 0.75), c + vec2(r * 0.8, 0.0), c + vec2(-r * 0.55, r * 0.75)];
            p.add(Shape::convex_polygon(pts, col, Stroke::NONE));
        }
        Icon::Plus => {
            p.line_segment([c + vec2(-r * 0.8, 0.0), c + vec2(r * 0.8, 0.0)], Stroke::new(r * 0.24, col));
            p.line_segment([c + vec2(0.0, -r * 0.8), c + vec2(0.0, r * 0.8)], Stroke::new(r * 0.24, col));
        }
        Icon::Cross => {
            let s = Stroke::new(r * 0.22, col);
            p.line_segment([c + vec2(-r * 0.6, -r * 0.6), c + vec2(r * 0.6, r * 0.6)], s);
            p.line_segment([c + vec2(-r * 0.6, r * 0.6), c + vec2(r * 0.6, -r * 0.6)], s);
        }
        Icon::Gear => {
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * r * 0.55, c + d * r * 0.95], Stroke::new(r * 0.26, col));
            }
            p.circle_stroke(c, r * 0.55, Stroke::new(r * 0.22, col));
        }
        Icon::Refresh | Icon::Power => {
            let (start, end) = if kind == Icon::Power { (-1.1f32, 4.25f32) } else { (0.3f32, 5.6f32) };
            let n = 28;
            let pts: Vec<Pos2> = (0..=n)
                .map(|i| {
                    let a = start + (end - start) * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
                    c + vec2(a.cos(), a.sin()) * r * 0.75
                })
                .collect();
            p.add(Shape::line(pts.clone(), st));
            if kind == Icon::Power {
                p.line_segment([c + vec2(0.0, -r * 0.95), c + vec2(0.0, -r * 0.15)], st);
            } else {
                let tip = *pts.last().unwrap();
                p.add(Shape::convex_polygon(
                    vec![tip + vec2(-r * 0.3, -r * 0.25), tip + vec2(r * 0.3, -r * 0.05), tip + vec2(-r * 0.1, r * 0.3)],
                    col,
                    Stroke::NONE,
                ));
            }
        }
        Icon::Doc => {
            let rect = Rect::from_center_size(c, vec2(r * 1.2, r * 1.6));
            p.rect_stroke(rect, r * 0.1, st, egui::StrokeKind::Middle);
            for i in 0..3 {
                let y = rect.top() + r * (0.45 + 0.35 * i as f32);
                p.line_segment([pos2(rect.left() + r * 0.25, y), pos2(rect.right() - r * 0.25, y)], st);
            }
        }
        Icon::Expand => {
            let k = r * 0.8;
            let l = r * 0.45;
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let corner = c + vec2(sx * k, sy * k);
                p.line_segment([corner, corner + vec2(-sx * l, 0.0)], st);
                p.line_segment([corner, corner + vec2(0.0, -sy * l)], st);
            }
        }
        Icon::Grid => {
            let s = r * 0.62;
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let rc = Rect::from_center_size(c + vec2(dx * s * 0.62, dy * s * 0.62), vec2(s * 0.95, s * 0.95));
                p.rect_filled(rc, r * 0.08, col);
            }
        }
        Icon::Trophy => {
            // Cup + handles + stem + base.
            let cup = vec![
                c + vec2(-r * 0.62, -r * 0.8),
                c + vec2(r * 0.62, -r * 0.8),
                c + vec2(r * 0.42, -r * 0.05),
                c + vec2(0.0, r * 0.18),
                c + vec2(-r * 0.42, -r * 0.05),
            ];
            p.add(Shape::convex_polygon(cup, col, Stroke::NONE));
            let hs = Stroke::new(r * 0.16, col);
            p.circle_stroke(c + vec2(-r * 0.62, -r * 0.45), r * 0.28, hs);
            p.circle_stroke(c + vec2(r * 0.62, -r * 0.45), r * 0.28, hs);
            p.rect_filled(Rect::from_center_size(c + vec2(0.0, r * 0.4), vec2(r * 0.2, r * 0.45)), 0.0, col);
            p.rect_filled(Rect::from_center_size(c + vec2(0.0, r * 0.75), vec2(r * 0.8, r * 0.22)), r * 0.05, col);
        }
        Icon::Disc => {
            p.circle_filled(c, r, col);
            p.circle_filled(c, r * 0.3, Color32::from_black_alpha(120));
            p.circle_stroke(c, r * 0.65, Stroke::new(r * 0.06, Color32::from_black_alpha(40)));
        }
        Icon::Palette => {
            for (i, (dx, dy)) in [(-0.45, -0.3), (0.0, -0.55), (0.45, -0.3), (0.35, 0.3), (-0.35, 0.3)].iter().enumerate() {
                let cc = [Color32::from_rgb(140, 210, 60), Color32::from_rgb(80, 160, 240), Color32::from_rgb(240, 160, 60), Color32::from_rgb(180, 120, 230), Color32::from_gray(90)][i];
                p.circle_filled(c + vec2(dx * r, dy * r), r * 0.3, cc.gamma_multiply(col.a() as f32 / 255.0));
            }
        }
        Icon::Image => {
            let rect = Rect::from_center_size(c, vec2(r * 1.7, r * 1.3));
            p.rect_stroke(rect, r * 0.1, st, egui::StrokeKind::Middle);
            p.add(Shape::convex_polygon(
                vec![rect.left_bottom() + vec2(r * 0.2, -r * 0.2), rect.center() + vec2(-r * 0.1, -r * 0.05), rect.right_bottom() + vec2(-r * 0.6, -r * 0.2)],
                col,
                Stroke::NONE,
            ));
            p.circle_filled(rect.right_top() + vec2(-r * 0.45, r * 0.4), r * 0.16, col);
        }
        Icon::Info => {
            p.circle_stroke(c, r * 0.85, st);
            p.circle_filled(c + vec2(0.0, -r * 0.4), r * 0.12, col);
            p.line_segment([c + vec2(0.0, -r * 0.12), c + vec2(0.0, r * 0.5)], Stroke::new(r * 0.18, col));
        }
    }
}
