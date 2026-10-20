//! Versus games for linked devices. Both sides get the same seed, play the identical round,
//! and compare scores.

use super::{Kit, MiniGame, Spec};
use crate::content::{Content, Pose};
use crate::lcd::Lcd;
use crate::{Button, Tune};

// ====================================================================== Eats
/// Mash B for six seconds.
#[derive(Default)]
pub struct Eats {
    score: u32,
}

impl MiniGame for Eats {
    fn spec(&self) -> Spec {
        Spec::new("EATS!", "MASH B", 6.0, 0, 60)
    }
    fn press(&mut self, _b: Button, _k: &mut Kit) {
        self.score += 1;
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, pet: &str, _t: f64) {
        let pose = if self.score % 2 == 1 { Pose::Eat } else { Pose::Idle1 };
        lcd.draw(c.sprite(pet, pose), 0, 0);
        let meal = c.prop("meal");
        lcd.blit(meal, 17, 8, false, false, Some((8 - self.score as i32 / 6).max(1)));
        lcd.text(&self.score.to_string(), 18, 1);
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Flags
/// Twelve arrows: A for left, B for up, C for right, quicker each time.
#[derive(Default)]
pub struct Flags {
    dirs: Vec<usize>,
    k: usize,
    rt: f64,
    fb: f64,
    score: u32,
}

impl Flags {
    fn next(&mut self, ok: bool, k: &mut Kit) {
        if ok {
            self.score += 1;
            k.good();
        } else {
            k.play(Tune::No);
        }
        self.fb = 0.3;
        self.k += 1;
        self.rt = 1.3 - self.k as f64 * 0.04;
        if self.k >= self.dirs.len() {
            k.end();
        }
    }
}

impl MiniGame for Flags {
    fn spec(&self) -> Spec {
        Spec::new("FLAGS", "A=L B=UP C=R", 999.0, 0, 12)
    }
    fn start(&mut self, k: &mut Kit) {
        self.dirs = (0..12).map(|_| k.rng.below(3)).collect();
        self.rt = 1.3;
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        if self.fb > 0.0 || self.k >= self.dirs.len() {
            return;
        }
        let ok = b.index() == self.dirs[self.k];
        self.next(ok, k);
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        if self.fb > 0.0 {
            self.fb -= dt;
            return;
        }
        if self.k >= self.dirs.len() {
            return;
        }
        self.rt -= dt;
        if self.rt <= 0.0 {
            self.next(false, k);
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        if self.fb > 0.0 || self.k >= self.dirs.len() {
            return;
        }
        let arrow = ["arrow_l", "arrow_u", "arrow_r"][self.dirs[self.k]];
        lcd.draw(c.prop(arrow), 13, 3);
        lcd.text(&self.score.to_string(), 0, 1);
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Bump
/// A bar sweeps back and forth; press B while it's in the box. Five tries.
#[derive(Default)]
pub struct Bump {
    tries: u32,
    pos: f64,
    dir: f64,
    speed: f64,
    pause: f64,
    score: u32,
}

impl MiniGame for Bump {
    fn spec(&self) -> Spec {
        Spec::new("BUMP", "B IN THE MIDDLE", 999.0, 0, 25)
    }
    fn start(&mut self, k: &mut Kit) {
        self.tries = 5;
        self.dir = 1.0;
        self.speed = k.rng.uniform(26.0, 34.0);
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        if b != Button::B || self.pause > 0.0 || self.tries == 0 {
            return;
        }
        let pts = (5 - ((self.pos - 15.5).abs() / 2.0) as i32).max(0) as u32;
        self.score += pts;
        k.play(if pts >= 4 { Tune::Ok } else { Tune::Tick });
        self.tries -= 1;
        self.pause = 0.6;
        if self.tries == 0 {
            k.end();
        }
    }
    fn tick(&mut self, dt: f64, _k: &mut Kit) {
        if self.pause > 0.0 {
            self.pause -= dt;
            return;
        }
        self.pos += self.dir * self.speed * dt;
        if self.pos >= 31.0 {
            self.pos = 31.0;
            self.dir = -1.0;
        } else if self.pos <= 0.0 {
            self.pos = 0.0;
            self.dir = 1.0;
        }
    }
    fn draw(&self, lcd: &mut Lcd, _c: &Content, _pet: &str, _t: f64) {
        lcd.rect(13, 10, 6, 5, false);
        lcd.vline(self.pos as i32, 9, 15);
        lcd.text(&self.score.to_string(), 0, 1);
        for i in 0..self.tries as i32 {
            lcd.dot(31 - i * 2, 1);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}
