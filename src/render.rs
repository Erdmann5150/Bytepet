//! Draws the device with tiny-skia into a transparent RGBA pixmap (premultiplied alpha).
//! The static shell is cached per (look, size); buttons, icons, the LCD and the settings
//! menu are drawn every frame.

use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, Mask, Paint, Path, PathBuilder, Pixmap, Point, RadialGradient, Rect,
    SpreadMode, Stroke, Transform,
};

use crate::core::Core;
use crate::font::{for_each_dot, text_width};
use crate::layout::*;
use crate::lcd::{Lcd, H as LH, W as LW};
use crate::look::{Antenna, ButtonShape, Corners, Look, Pattern};
use crate::overlay::{self, Env, Page};
use crate::rng::Pcg32;
use crate::sprite::Sprite;

fn rgba(c: [u8; 3], a: u8) -> Color {
    Color::from_rgba8(c[0], c[1], c[2], a)
}

fn paint(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    p
}

fn poly(points: &[(f32, f32)]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let (x0, y0) = points.first()?;
    pb.move_to(*x0, *y0);
    for &(x, y) in &points[1..] {
        pb.line_to(x, y);
    }
    pb.close();
    pb.finish()
}

fn circle(cx: f32, cy: f32, r: f32) -> Option<Path> {
    PathBuilder::from_circle(cx, cy, r)
}

fn oval(x: f32, y: f32, w: f32, h: f32) -> Option<Path> {
    Rect::from_xywh(x, y, w, h).and_then(PathBuilder::from_oval)
}

fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let k = 0.5523 * r;
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    pb.close();
    pb.finish()
}

fn fill(pm: &mut Pixmap, path: Option<Path>, c: Color, ts: Transform, mask: Option<&Mask>) {
    if let Some(p) = path {
        pm.fill_path(&p, &paint(c), FillRule::Winding, ts, mask);
    }
}

fn stroke(pm: &mut Pixmap, path: Option<Path>, c: Color, width: f32, ts: Transform) {
    if let Some(p) = path {
        let s = Stroke { width, line_cap: LineCap::Round, ..Stroke::default() };
        pm.stroke_path(&p, &paint(c), &s, ts, None);
    }
}

#[allow(clippy::too_many_arguments)]
/// Pixel-font text. `s` is the size of one font dot in the units of `ts`.
fn ptext(pm: &mut Pixmap, text: &str, x: f32, y: f32, s: f32, c: Color, center: bool, ts: Transform) {
    let x = if center { x - text_width(text) as f32 * s / 2.0 } else { x };
    let p = paint(c);
    for_each_dot(text, |dx, dy| {
        if let Some(r) = Rect::from_xywh(x + dx as f32 * s, y + dy as f32 * s, s, s) {
            pm.fill_rect(r, &p, ts, None);
        }
    });
}

#[derive(Default)]
pub struct Renderer {
    shell_cache: Option<(Look, u32, u32, Pixmap)>,
}

/// Window size in pixels for a given scale (window pixels per design unit).
pub fn window_size(scale: f32) -> (u32, u32) {
    ((VIEW.2 * scale).round() as u32, (VIEW.3 * scale).round() as u32)
}

fn design_ts(scale: f32) -> Transform {
    Transform::from_row(scale, 0.0, 0.0, scale, -VIEW.0 * scale, -VIEW.1 * scale)
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer::default()
    }

    /// Render the whole widget. `scale` is window pixels per design unit (size setting × DPI);
    /// `env` is needed only while the menu is open. The menu sizes itself to the window, so
    /// `_dpi` is no longer used; it stays so callers don't change.
    pub fn render(&mut self, core: &Core, scale: f32, _dpi: f32, env: Option<&Env>, wall: f64) -> Pixmap {
        let (w, h) = window_size(scale);
        let mut pm = self.shell(&core.game.settings.look, w.max(1), h.max(1), scale).clone();
        self.dynamic(&mut pm, core, scale, wall);
        if core.overlay.is_open() {
            if let Some(env) = env {
                draw_overlay(&mut pm, core, env);
            }
        }
        pm
    }

    fn shell(&mut self, look: &Look, w: u32, h: u32, scale: f32) -> &Pixmap {
        let fresh = !matches!(&self.shell_cache, Some((l, cw, ch, _)) if l == look && *cw == w && *ch == h);
        if fresh {
            let mut pm = Pixmap::new(w, h).expect("pixmap");
            paint_shell(&mut pm, look, scale);
            self.shell_cache = Some((*look, w, h, pm));
        }
        &self.shell_cache.as_ref().unwrap().3
    }

    fn dynamic(&self, pm: &mut Pixmap, core: &Core, scale: f32, wall: f64) {
        let ts = design_ts(scale);
        let look = &core.game.settings.look;
        let button = look.button_color();
        // buttons
        for &(b, bx, by) in &BUTTONS {
            let down = core.held[b.index()];
            let y = by + if down { 3.0 } else { 0.0 };
            let shape = || button_path(look.button_shape, bx, y, BTN_R);
            fill(pm, shape(), rgba(lighten(button, if down { 0.3 } else { 0.55 }), 255), ts, None);
            stroke(pm, shape(), rgba(darken(button, 0.3), 255), 1.5, ts);
            if !down {
                let (hx, hy, hr) = match look.button_shape {
                    ButtonShape::Star => (bx - 4.0, by - 5.0, 4.0),
                    _ => (bx - 7.0, by - 8.0, 6.0),
                };
                fill(pm, circle(hx, hy, hr), Color::from_rgba8(255, 255, 255, 220), ts, None);
            }
        }
        // icons
        let states = core.icon_states(wall);
        let rects = icon_rects();
        for (i, icon) in core.content.icons.iter().enumerate() {
            let (rx, ry, _, _) = rects[i];
            let c = if states[i] { Color::from_rgba8(40, 42, 40, 255) } else { Color::from_rgba8(200, 198, 190, 255) };
            draw_sprite(pm, icon, rx, ry, ICON_SCALE, c, ts);
        }
        // LCD dots, aligned to whole pixels so they stay crisp at any size
        let (_, lcd_off, lcd_on) = look.lcd.colors();
        let (lx, ly) = LCD_POS;
        let cell_px = CELL * scale;
        let gap = if cell_px >= 4.0 { (scale).clamp(0.5, 1.5) } else { 0.0 };
        let ox = (lx - VIEW.0) * scale;
        let oy = (ly - VIEW.1) * scale;
        for y in 0..LH {
            for x in 0..LW {
                let v = core.shade[(y * LW + x) as usize].clamp(0.0, 1.0);
                let c = [0, 1, 2].map(|k| (lcd_off[k] as f32 + (lcd_on[k] as f32 - lcd_off[k] as f32) * v) as u8);
                let x0 = (ox + x as f32 * cell_px).round();
                let y0 = (oy + y as f32 * cell_px).round();
                let x1 = (ox + (x + 1) as f32 * cell_px).round() - gap;
                let y1 = (oy + (y + 1) as f32 * cell_px).round() - gap;
                if let Some(r) = Rect::from_ltrb(x0, y0, x1.max(x0 + 1.0), y1.max(y0 + 1.0)) {
                    pm.fill_rect(r, &paint(rgba(c, 255)), Transform::identity(), None);
                }
            }
        }
        let tags = core.status_tags();
        if !tags.is_empty() {
            ptext(pm, &tags, FACE.0 + FACE.2 - 34.0, FACE.1 + 14.0, 2.0, Color::from_rgba8(200, 60, 60, 255), true, ts);
        }
        if core.wants_text() {
            let y = FACE.1 + FACE.3 + 30.0;
            ptext(pm, "TYPE ON KEYBOARD", CX, y, 2.0, Color::WHITE, true, ts);
        }
    }
}

fn draw_sprite(pm: &mut Pixmap, s: &Sprite, x: f32, y: f32, dot: f32, c: Color, ts: Transform) {
    let p = paint(c);
    for sy in 0..s.h {
        for sx in 0..s.w {
            if s.get(sx, sy) {
                if let Some(r) = Rect::from_xywh(x + sx as f32 * dot, y + sy as f32 * dot, dot, dot) {
                    pm.fill_rect(r, &p, ts, None);
                }
            }
        }
    }
}

/// A button's outline, `r` being the round button's radius.
fn button_path(shape: ButtonShape, cx: f32, cy: f32, r: f32) -> Option<Path> {
    match shape {
        ButtonShape::Round => circle(cx, cy, r),
        ButtonShape::Square => rounded(cx - 0.9 * r, cy - 0.9 * r, 1.8 * r, 1.8 * r, 0.4 * r),
        ButtonShape::Gem => poly(&[(cx, cy - 1.15 * r), (cx + 1.05 * r, cy), (cx, cy + 1.15 * r), (cx - 1.05 * r, cy)]),
        ButtonShape::Heart => poly(&heart_points(cx, cy + 1.0, 1.12 * r, 0.0)),
        ButtonShape::Star => poly(&star_points_at(cx, cy + 2.0, 1.25 * r, 0.62 * r, 0.0)),
    }
}

/// The static parts of the device.
fn paint_shell(pm: &mut Pixmap, look: &Look, scale: f32) {
    let ts = design_ts(scale);
    let base = look.body;
    let rim = darken(base, 0.35);

    // key ring and antenna, behind the shell
    if look.ring {
        stroke(pm, circle(RING.0, RING.1, 16.0), rgba([190, 190, 200], 255), 4.0, ts);
    }
    let antenna = antenna_points(look.antenna);
    if !antenna.is_empty() {
        let mut pb = PathBuilder::new();
        pb.move_to(STAR.0 + 12.0, STAR.1 + 14.0);
        pb.line_to(STAR.0 + 48.0, STAR.1 + 52.0);
        stroke(pm, pb.finish(), rgba([200, 200, 210], 255), 6.0, ts);
        let color = look.antenna_color.unwrap_or(lighten(base, 0.45));
        fill(pm, poly(&antenna), rgba(color, 215), ts, None);
        stroke(pm, poly(&antenna), Color::WHITE, 2.0, ts);
        // a highlight: the outline shrunk towards its middle
        let (mx, my) = if look.antenna == Antenna::Star {
            STAR
        } else {
            let n = antenna.len() as f32;
            antenna.iter().fold((0.0, 0.0), |(sx, sy), &(x, y)| (sx + x / n, sy + y / n))
        };
        let inner: Vec<(f32, f32)> = antenna.iter().map(|&(x, y)| (x - 0.4 * (x - mx), y - 0.4 * (y - my))).collect();
        fill(pm, poly(&inner), Color::from_rgba8(255, 255, 255, 70), ts, None);
    }

    // the shell: dark rim, then a softly lit plastic body
    fill(pm, poly(&outer_egg()), rgba(rim, 255), ts, None);
    let inner = inner_egg();
    let stops = vec![
        GradientStop::new(0.0, rgba(lighten(base, 0.35), 255)),
        GradientStop::new(0.5, rgba(base, 255)),
        GradientStop::new(1.0, rgba(darken(base, 0.15), 255)),
    ];
    let center = Point::from_xy(CX - 70.0, CY - 140.0);
    if let (Some(path), Some(shader)) =
        (poly(&inner), RadialGradient::new(center, 0.0, center, 420.0, stops, SpreadMode::Pad, Transform::identity()))
    {
        let p = Paint { shader, anti_alias: true, ..Default::default() };
        pm.fill_path(&path, &p, FillRule::Winding, ts, None);
    }

    // pattern and gloss, clipped to the body
    let mut mask = Mask::new(pm.width(), pm.height()).expect("mask");
    if let Some(path) = poly(&inner) {
        mask.fill_path(&path, FillRule::Winding, true, ts);
    }
    paint_pattern(pm, look, ts, &mask);
    if look.gloss {
        fill(pm, oval(CX - 150.0, CY - 238.0, 170.0, 90.0), Color::from_rgba8(255, 255, 255, 70), ts, Some(&mask));
        fill(pm, oval(CX + 110.0, CY + 40.0, 40.0, 140.0), Color::from_rgba8(255, 255, 255, 40), ts, Some(&mask));
    }

    // faceplate and screen
    let (fx, fy, fw, fh) = FACE;
    let r = look.corners.radius();
    let edge = look.edge.unwrap_or(darken(base, 0.45));
    fill(pm, rounded(fx - 5.0, fy - 5.0, fw + 10.0, fh + 10.0, r + 4.0), rgba(edge, 255), ts, None);
    fill(pm, rounded(fx, fy, fw, fh, r), rgba(look.face, 255), ts, None);
    stroke(pm, rounded(fx, fy, fw, fh, r), rgba(darken(look.face, 0.09), 255), 2.0, ts);
    let (lx, ly) = LCD_POS;
    let (lw, lh) = (LW as f32 * CELL, LH as f32 * CELL);
    let k = if look.corners == Corners::Square { 0.4 } else { 1.0 };
    fill(pm, rounded(lx - 10.0, ly - 10.0, lw + 20.0, lh + 20.0, 8.0 * k), rgba(look.frame, 255), ts, None);
    fill(pm, rounded(lx - 6.0, ly - 6.0, lw + 12.0, lh + 12.0, 5.0 * k), rgba(look.lcd.colors().0, 255), ts, None);

    for &(b, bx, by) in &BUTTONS {
        fill(pm, button_path(look.button_shape, bx, by + 4.0, BTN_R + 4.0), rgba(darken(base, 0.5), 255), ts, None);
        if look.letters {
            let label = match b {
                crate::Button::A => "A",
                crate::Button::B => "B",
                crate::Button::C => "C",
            };
            // pointed shapes reach further down
            let y = by + BTN_R + if look.button_shape == ButtonShape::Round { 12.0 } else { 16.0 };
            // printed darker than the body, or lighter on a dark one
            let ink = if luma(base) < 0.3 { lighten(base, 0.55) } else { darken(base, 0.55) };
            ptext(pm, label, bx, y, 3.0, rgba(ink, 255), true, ts);
        }
    }
    // the name is lighter than the body, or darker on a pale one
    let name = if luma(base) > 0.75 { darken(base, 0.4) } else { lighten(base, 0.6) };
    ptext(pm, "BYTEPET", CX, fy + fh + 14.0, 2.0, rgba(name, 255), true, ts);
}

/// The body's pattern, drawn through `mask`.
fn paint_pattern(pm: &mut Pixmap, look: &Look, ts: Transform, mask: &Mask) {
    let mask = Some(mask);
    let mut rng = Pcg32::new(look.pattern.seed());
    let (w, h) = (CANVAS_W, CANVAS_H);
    let accent = rgba(look.accent_color(), 255);
    match look.pattern {
        Pattern::Plain => {}
        Pattern::Zebra => {
            for i in -8..14 {
                let x0 = i as f32 * 40.0;
                let wob = rng.range(-20, 20) as f32;
                let pts = [(x0, 0.0), (x0 + 16.0, 0.0), (x0 + 136.0 + wob, h), (x0 + 120.0, h)];
                fill(pm, poly(&pts), accent, ts, mask);
            }
        }
        Pattern::Flame => {
            for i in 0..9 {
                let x = 40.0 + i as f32 * 45.0;
                let fh = rng.range(140, 260) as f32;
                let pts = [
                    (x - 26.0, h),
                    (x + 26.0, h),
                    (x + 12.0, h - fh * 0.5),
                    (x + 4.0, h - fh),
                    (x - 10.0, h - fh * 0.55),
                ];
                fill(pm, poly(&pts), accent, ts, mask);
            }
        }
        Pattern::Sprinkles => {
            let cols = [[255, 255, 255], [80, 200, 255], [255, 230, 70], [140, 90, 230], [90, 220, 120]];
            for _ in 0..160 {
                let (x, y) = (rng.range(0, w as i64) as f32, rng.range(0, h as i64) as f32);
                let a = rng.uniform(0.0, std::f64::consts::PI) as f32;
                // an accent colour makes every sprinkle that colour
                let c = look.accent.unwrap_or(cols[rng.below(cols.len())]);
                let mut pb = PathBuilder::new();
                pb.move_to(x, y);
                pb.line_to(x + 9.0 * a.cos(), y + 9.0 * a.sin());
                if let Some(path) = pb.finish() {
                    let s = Stroke { width: 4.0, line_cap: LineCap::Round, ..Stroke::default() };
                    pm.stroke_path(&path, &paint(rgba(c, 255)), &s, ts, mask);
                }
            }
        }
        Pattern::Camo => {
            let cols = match look.accent {
                None => [[70, 86, 52], [150, 140, 96], [52, 60, 40]],
                Some(a) => [darken(a, 0.25), lighten(a, 0.3), darken(a, 0.5)],
            };
            for _ in 0..60 {
                let (x, y) = (rng.range(0, w as i64) as f32, rng.range(0, h as i64) as f32);
                let (ew, eh) = (rng.range(30, 80) as f32, rng.range(20, 50) as f32);
                let c = cols[rng.below(cols.len())];
                fill(pm, oval(x, y, ew, eh), rgba(c, 255), ts, mask);
            }
        }
        Pattern::Dots => {
            for row in 0..16 {
                for col in 0..12 {
                    let x = col as f32 * 44.0 + if row % 2 == 1 { 22.0 } else { 0.0 };
                    fill(pm, circle(x, 30.0 + row as f32 * 40.0, 10.0), accent, ts, mask);
                }
            }
        }
        Pattern::Stripes => {
            for i in 0..13 {
                let y = 40.0 + i as f32 * 56.0;
                let pts = [(0.0, y), (w, y - 40.0), (w, y - 16.0), (0.0, y + 24.0)];
                fill(pm, poly(&pts), accent, ts, mask);
            }
        }
        Pattern::Hearts => {
            for row in 0..11 {
                for col in 0..8 {
                    let x = 20.0 + col as f32 * 60.0 + if row % 2 == 1 { 30.0 } else { 0.0 };
                    let tilt = if (row + col) % 2 == 0 { -0.3 } else { 0.3 };
                    fill(pm, poly(&heart_points(x, 40.0 + row as f32 * 58.0, 13.0, tilt)), accent, ts, mask);
                }
            }
        }
    }
}

fn draw_overlay(pm: &mut Pixmap, core: &Core, env: &Env) {
    let p = core.overlay.panel(pm.width() as f32, pm.height() as f32);
    let id = Transform::identity();
    let pink = Color::from_rgba8(255, 140, 200, 255);
    let white = Color::from_rgba8(238, 238, 244, 255);
    let dim = Color::from_rgba8(150, 152, 170, 255);
    let fs = p.fs;
    let row_text_y = |i: usize| p.top + i as f32 * p.row_h + fs;
    if !core.overlay.is_open() {
        return;
    }
    let title = core.overlay.title();
    fill(pm, rounded(p.x, p.y, p.w, p.h, 3.0 * fs), Color::from_rgba8(24, 26, 36, 240), id, None);
    ptext(pm, title, p.x + 3.0 * fs, p.y + 2.0 * fs, fs, pink, false, id);
    match core.overlay.page {
        Page::Closed => {}
        Page::Menu | Page::Shell | Page::Part(_) => {
            for (i, it) in overlay::items(core, env).iter().enumerate() {
                if core.overlay.hover == Some(i) {
                    fill(
                        pm,
                        Rect::from_xywh(p.x + fs, p.top + i as f32 * p.row_h, p.w - 2.0 * fs, p.row_h)
                            .map(PathBuilder::from_rect),
                        Color::from_rgba8(236, 72, 153, 200),
                        id,
                        None,
                    );
                }
                ptext(pm, it.label, p.x + 3.0 * fs, row_text_y(i), fs, white, false, id);
                if !it.value.is_empty() {
                    let vx = p.x + p.w - 3.0 * fs - text_width(&it.value) as f32 * fs;
                    ptext(pm, &it.value, vx, row_text_y(i), fs, dim, false, id);
                }
            }
        }
        Page::Help => {
            let (_, lines) = overlay::HELP_PAGES[core.overlay.help.min(overlay::HELP_PAGES.len() - 1)];
            for (i, l) in lines.iter().enumerate() {
                ptext(pm, l, p.x + 3.0 * fs, row_text_y(i), fs, white, false, id);
            }
        }
    }
}

// ====================================================================== exports for docs/tests
/// A sprite on an LCD-coloured background, `cell` pixels per dot.
pub fn sprite_pixmap(s: &Sprite, cell: u32, pad: u32) -> Pixmap {
    let (w, h) = ((s.w as u32 + 2 * pad) * cell, (s.h as u32 + 2 * pad) * cell);
    let mut pm = Pixmap::new(w, h).unwrap();
    pm.fill(rgba(LCD_BG, 255));
    let gap = if cell >= 4 { 1.0 } else { 0.0 };
    for y in 0..s.h + 2 * pad as i32 {
        for x in 0..s.w + 2 * pad as i32 {
            let on = s.get(x - pad as i32, y - pad as i32);
            if let Some(r) = Rect::from_xywh(
                (x as u32 * cell) as f32,
                (y as u32 * cell) as f32,
                cell as f32 - gap,
                cell as f32 - gap,
            ) {
                pm.fill_rect(r, &paint(rgba(if on { LCD_ON } else { LCD_OFF }, 255)), Transform::identity(), None);
            }
        }
    }
    pm
}

/// The LCD contents as an image.
pub fn lcd_pixmap(lcd: &Lcd, cell: u32) -> Pixmap {
    let rows: Vec<String> = (0..LH).map(|y| (0..LW).map(|x| if lcd.get(x, y) { '#' } else { '.' }).collect()).collect();
    sprite_pixmap(&Sprite::from_rows(&rows), cell, 1)
}
