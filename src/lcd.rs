//! The 32x16 dot-matrix LCD framebuffer.

use crate::font::{for_each_dot, text_width};
use crate::sprite::Sprite;

pub const W: i32 = 32;
pub const H: i32 = 16;

#[derive(Clone)]
pub struct Lcd {
    pub buf: [u8; (W * H) as usize],
}

impl Default for Lcd {
    fn default() -> Self {
        Lcd { buf: [0; (W * H) as usize] }
    }
}

impl Lcd {
    pub fn clear(&mut self, v: u8) {
        self.buf.fill(v);
    }

    pub fn set(&mut self, x: i32, y: i32, v: u8) {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            self.buf[(y * W + x) as usize] = v;
        }
    }

    pub fn dot(&mut self, x: i32, y: i32) {
        self.set(x, y, 1);
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        (0..W).contains(&x) && (0..H).contains(&y) && self.buf[(y * W + x) as usize] != 0
    }

    /// Draw a sprite. `opaque` blanks its silhouette first; `clip_w` hides columns >= clip_w.
    pub fn blit(&mut self, s: &Sprite, x: i32, y: i32, flip: bool, opaque: bool, clip_w: Option<i32>) {
        let cw = clip_w.unwrap_or(s.w);
        for sy in 0..s.h {
            for sx in 0..s.w.min(cw) {
                let i = (sy * s.w + sx) as usize;
                let dx = if flip { s.w - 1 - sx } else { sx };
                if opaque && s.mask[i] && !s.pix[i] {
                    self.set(x + dx, y + sy, 0);
                }
                if s.pix[i] {
                    self.set(x + dx, y + sy, 1);
                }
            }
        }
    }

    pub fn draw(&mut self, s: &Sprite, x: i32, y: i32) {
        self.blit(s, x, y, false, false, None);
    }

    pub fn text(&mut self, s: &str, x: i32, y: i32) {
        // Text that starts on screen must fit; scrolling text goes through `line`.
        debug_assert!(x < 0 || x + text_width(s) <= W, "text {s:?} at x={x} runs off the LCD");
        for_each_dot(s, |dx, dy| self.set(x + dx, y + dy, 1));
    }

    pub fn text_center(&mut self, s: &str, y: i32) {
        self.text(s, (W - text_width(s)) / 2, y);
    }

    /// One line of up to 8 characters; longer text scrolls like a marquee.
    pub fn line(&mut self, s: &str, y: i32, t: f64, center: bool) {
        let w = text_width(s);
        if w <= W {
            self.text(s, if center { (W - w) / 2 } else { 0 }, y);
            return;
        }
        let span = w - W;
        let cyc = span + 16;
        let k = (((t - 0.7).max(0.0) * 9.0) as i32) % cyc;
        let x = -k.min(span);
        for_each_dot(s, |dx, dy| self.set(x + dx, y + dy, 1));
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, fill: bool) {
        for yy in y..y + h {
            for xx in x..x + w {
                if fill || yy == y || yy == y + h - 1 || xx == x || xx == x + w - 1 {
                    self.set(xx, yy, 1);
                }
            }
        }
    }

    pub fn clear_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, 0);
            }
        }
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32) {
        for x in x0..=x1 {
            self.set(x, y, 1);
        }
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32) {
        for y in y0..=y1 {
            self.set(x, y, 1);
        }
    }

    pub fn invert(&mut self) {
        for v in self.buf.iter_mut() {
            *v ^= 1;
        }
    }
}
