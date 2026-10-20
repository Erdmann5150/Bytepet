//! Device geometry and shell designs, in design units on a 440x640 canvas.

pub const CANVAS_W: f32 = 440.0;
pub const CANVAS_H: f32 = 640.0;
pub const CX: f32 = 220.0;
pub const CY: f32 = 318.0;
pub const SHELL_A: f32 = 196.0;
pub const SHELL_B: f32 = 262.0;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Plain,
    Zebra,
    Flame,
    Sprinkles,
    Camo,
}

#[derive(Clone, Copy, Debug)]
pub struct Shell {
    pub name: &'static str,
    pub base: [u8; 3],
    pub pattern: Pattern,
    pub button: [u8; 3],
}

pub const SHELLS: [Shell; 8] = [
    Shell { name: "BERRY", base: [236, 72, 153], pattern: Pattern::Plain, button: [236, 72, 153] },
    Shell { name: "LIME", base: [132, 204, 22], pattern: Pattern::Plain, button: [132, 204, 22] },
    Shell { name: "OCEAN", base: [56, 152, 236], pattern: Pattern::Plain, button: [56, 152, 236] },
    Shell { name: "GRAPE", base: [147, 92, 226], pattern: Pattern::Plain, button: [147, 92, 226] },
    Shell { name: "ZEBRA", base: [238, 238, 232], pattern: Pattern::Zebra, button: [236, 72, 153] },
    Shell { name: "FLAME", base: [249, 115, 22], pattern: Pattern::Flame, button: [249, 115, 22] },
    Shell { name: "DONUT", base: [246, 170, 200], pattern: Pattern::Sprinkles, button: [246, 170, 200] },
    Shell { name: "CAMO", base: [112, 132, 84], pattern: Pattern::Camo, button: [112, 132, 84] },
];

pub fn shell(i: u32) -> &'static Shell {
    &SHELLS[i as usize % SHELLS.len()]
}

pub fn lighten(c: [u8; 3], f: f32) -> [u8; 3] {
    c.map(|v| (v as f32 + (255.0 - v as f32) * f) as u8)
}

pub fn darken(c: [u8; 3], f: f32) -> [u8; 3] {
    c.map(|v| (v as f32 * (1.0 - f)) as u8)
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

pub fn star_points() -> Vec<(f32, f32)> {
    (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 34.0 } else { 15.0 };
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0 - 0.35;
            (STAR.0 + r * a.cos(), STAR.1 + r * a.sin())
        })
        .collect()
}

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

/// Is a design-space point on the device (shell, star antenna or key ring)?
pub fn on_device(x: f32, y: f32) -> bool {
    point_in_poly(x, y, &egg_points(CX, CY, SHELL_A + 3.0, SHELL_B + 3.0, 120))
        || point_in_poly(x, y, &star_points())
        || ((x - CX).powi(2) + (y - (CY - SHELL_B - 6.0)).powi(2)).sqrt() <= 22.0
        || (x > STAR.0 - 4.0 && x < STAR.0 + 56.0 && y > STAR.1 - 4.0 && y < STAR.1 + 56.0 && (x - y).abs() < 14.0)
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
