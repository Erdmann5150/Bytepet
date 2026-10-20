//! Device geometry, in design units on a 440x640 canvas. How it is coloured and decorated is
//! in [`crate::look`].

use crate::look::{Antenna, Corners, Look};

pub const CANVAS_W: f32 = 440.0;
pub const CANVAS_H: f32 = 640.0;
pub const CX: f32 = 220.0;
pub const CY: f32 = 318.0;
pub const SHELL_A: f32 = 196.0;
pub const SHELL_B: f32 = 262.0;
/// The faceplate with round corners; see [`face_rect`].
pub const FACE: (f32, f32, f32, f32) = (58.0, 168.0, 324.0, 252.0);
pub const LCD_POS: (f32, f32) = (92.0, 228.0);
pub const CELL: f32 = 8.0;
pub const ICON_SCALE: f32 = 3.0;
pub const BUTTONS: [(crate::Button, f32, f32); 3] =
    [(crate::Button::A, 142.0, 482.0), (crate::Button::B, 220.0, 500.0), (crate::Button::C, 298.0, 482.0)];
pub const BTN_R: f32 = 25.0;
pub const STAR: (f32, f32) = (70.0, 70.0);
/// The part of the canvas the widget window shows: x, y, w, h.
pub const VIEW: (f32, f32, f32, f32) = (2.0, 26.0, 436.0, 560.0);

pub const LCD_BG: [u8; 3] = [178, 190, 166];
pub const LCD_OFF: [u8; 3] = [168, 180, 157];
pub const LCD_ON: [u8; 3] = [32, 38, 36];

pub fn lighten(c: [u8; 3], f: f32) -> [u8; 3] {
    c.map(|v| (v as f32 + (255.0 - v as f32) * f) as u8)
}

pub fn darken(c: [u8; 3], f: f32) -> [u8; 3] {
    c.map(|v| (v as f32 * (1.0 - f)) as u8)
}

/// Perceived brightness, 0 to 1.
pub fn luma(c: [u8; 3]) -> f32 {
    (0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32) / 255.0
}

/// The egg outline (slightly narrower at the top).
pub fn egg_points(cx: f32, cy: f32, a: f32, b: f32, n: usize) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / n as f32;
            let s = t.sin();
            (cx + a * t.cos() * (1.0 + 0.085 * s), cy + b * s)
        })
        .collect()
}

pub fn outer_egg() -> Vec<(f32, f32)> {
    egg_points(CX, CY, SHELL_A, SHELL_B, 120)
}

pub fn inner_egg() -> Vec<(f32, f32)> {
    egg_points(CX, CY + 2.0, SHELL_A - 9.0, SHELL_B - 9.0, 120)
}

/// A five-pointed star around (cx, cy), turned by `rot` radians.
pub fn star_points_at(cx: f32, cy: f32, outer: f32, inner: f32, rot: f32) -> Vec<(f32, f32)> {
    (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0 + rot;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

pub fn star_points() -> Vec<(f32, f32)> {
    star_points_at(STAR.0, STAR.1, 34.0, 15.0, -0.35)
}

/// A heart about `2 * r` wide around (cx, cy), turned by `rot` radians.
pub fn heart_points(cx: f32, cy: f32, r: f32, rot: f32) -> Vec<(f32, f32)> {
    let k = r / 16.0;
    let (s, c) = rot.sin_cos();
    (0..48)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / 48.0;
            let x = 16.0 * t.sin().powi(3);
            // the classic heart curve, flipped for y-down and centred on its middle
            let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos()) - 2.5;
            let (x, y) = (x * k, y * k);
            (cx + x * c - y * s, cy + x * s + y * c)
        })
        .collect()
}

/// Points on a circle.
pub fn circle_points(cx: f32, cy: f32, r: f32, n: usize) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / n as f32;
            (cx + r * t.cos(), cy + r * t.sin())
        })
        .collect()
}

/// The antenna's outline; empty for none. Every shape fits within the original star's reach,
/// so the window shape and hit-testing only have to follow the outline.
pub fn antenna_points(a: Antenna) -> Vec<(f32, f32)> {
    let (cx, cy) = STAR;
    let turn = |pts: Vec<(f32, f32)>, rot: f32| -> Vec<(f32, f32)> {
        let (s, c) = rot.sin_cos();
        pts.into_iter().map(|(x, y)| (cx + x * c - y * s, cy + x * s + y * c)).collect()
    };
    match a {
        Antenna::Star => star_points(),
        Antenna::Heart => heart_points(cx, cy, 30.0, -0.3),
        Antenna::Ball => circle_points(cx, cy, 25.0, 40),
        Antenna::Moon => {
            // a disc of radius 31 with one of radius 25 taken out of it, 16 units off-centre
            let (big, small, d) = (31.0f32, 25.0f32, 16.0f32);
            let ix = (d * d + big * big - small * small) / (2.0 * d);
            let iy = (big * big - ix * ix).sqrt();
            let a0 = iy.atan2(ix);
            let b0 = iy.atan2(ix - d);
            let n = 24;
            let mut pts = Vec::new();
            for i in 0..=n {
                let t = a0 + (std::f32::consts::TAU - 2.0 * a0) * i as f32 / n as f32;
                pts.push((big * t.cos() - 4.0, big * t.sin()));
            }
            for i in 0..=n {
                let t = (std::f32::consts::TAU - b0) - (std::f32::consts::TAU - 2.0 * b0) * i as f32 / n as f32;
                pts.push((d + small * t.cos() - 4.0, small * t.sin()));
            }
            turn(pts, -0.8)
        }
        Antenna::Bolt => {
            let pts = [(0.35, -1.0), (-0.6, 0.15), (-0.05, 0.15), (-0.35, 1.0), (0.65, -0.2), (0.1, -0.2)];
            turn(pts.iter().map(|&(x, y)| (x * 34.0, y * 34.0)).collect(), 0.35)
        }
        Antenna::None => Vec::new(),
    }
}

/// The faceplate (x, y, w, h) for a corner style, brought in at the sides so its corners stay
/// on the shell.
pub fn face_rect(c: Corners) -> (f32, f32, f32, f32) {
    let i = c.inset();
    (FACE.0 + i, FACE.1, FACE.2 - 2.0 * i, FACE.3)
}

/// The key ring at the top of the shell: centre and outer reach.
pub const RING: (f32, f32, f32) = (CX, CY - SHELL_B - 6.0, 22.0);

/// (x, y, w, h) of the ten icons: top row 0-4, bottom row 5-9.
pub fn icon_rects() -> [(f32, f32, f32, f32); 10] {
    let mut out = [(0.0, 0.0, 0.0, 0.0); 10];
    let (lx, ly) = LCD_POS;
    let step = 32.0 * CELL / 5.0;
    for row in 0..2 {
        let y = if row == 0 { ly - 46.0 } else { ly + 16.0 * CELL + 16.0 };
        for col in 0..5 {
            let x = (lx + step * col as f32 + step / 2.0 - 11.0 * ICON_SCALE / 2.0).floor();
            out[row * 5 + col] = (x, y, 11.0 * ICON_SCALE, 10.0 * ICON_SCALE);
        }
    }
    out
}

/// The LCD itself, a little padded: clicking it presses B.
pub fn on_lcd(x: f32, y: f32) -> bool {
    let (lx, ly) = LCD_POS;
    let pad = 6.0;
    x >= lx - pad && x <= lx + 32.0 * CELL + pad && y >= ly - pad && y <= ly + 16.0 * CELL + pad
}

pub fn icon_at(x: f32, y: f32) -> Option<usize> {
    icon_rects()
        .iter()
        .position(|&(rx, ry, rw, rh)| x >= rx - 5.0 && x <= rx + rw + 5.0 && y >= ry - 5.0 && y <= ry + rh + 5.0)
}

pub fn button_at(x: f32, y: f32) -> Option<crate::Button> {
    BUTTONS.iter().find(|&&(_, bx, by)| (x - bx).powi(2) + (y - by).powi(2) <= (BTN_R + 6.0).powi(2)).map(|b| b.0)
}

/// Is a design-space point on the device (shell, antenna or key ring)?
pub fn on_device(x: f32, y: f32, look: &Look) -> bool {
    let antenna = antenna_points(look.antenna);
    point_in_poly(x, y, &egg_points(CX, CY, SHELL_A + 3.0, SHELL_B + 3.0, 120))
        || (!antenna.is_empty() && point_in_poly(x, y, &antenna))
        || (look.ring && ((x - RING.0).powi(2) + (y - RING.1).powi(2)).sqrt() <= RING.2)
        || (!antenna.is_empty()
            && x > STAR.0 - 4.0
            && x < STAR.0 + 56.0
            && y > STAR.1 - 4.0
            && y < STAR.1 + 56.0
            && (x - y).abs() < 14.0)
}

pub fn point_in_poly(x: f32, y: f32, pts: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_faceplate_stays_on_the_shell_with_every_corner_style() {
        // the round faceplate's rim just reaches the shell's edge; the others may go no further
        let shell = egg_points(CX, CY, SHELL_A + 1.5, SHELL_B + 1.5, 240);
        for c in Corners::ALL {
            // the faceplate's rim, as drawn: 5 units round the faceplate, corners 4 rounder
            let (x, y, w, h) = face_rect(c);
            let (x, y, w, h, r) = (x - 5.0, y - 5.0, w + 10.0, h + 10.0, c.radius() + 4.0);
            for (cx, cy, a0) in [
                (x + r, y + r, 180.0f32),
                (x + w - r, y + r, 270.0),
                (x + w - r, y + h - r, 0.0),
                (x + r, y + h - r, 90.0),
            ] {
                for k in 0..=18 {
                    let a = (a0 + k as f32 * 5.0).to_radians();
                    let (px, py) = (cx + r * a.cos(), cy + r * a.sin());
                    assert!(point_in_poly(px, py, &shell), "{c:?}: ({px:.1}, {py:.1}) is off the shell");
                }
            }
        }
    }
}
