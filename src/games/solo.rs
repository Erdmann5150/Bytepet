//! The four everyday games: Climb (humor), Tug-o-war (gorgeous), Apples (passion), Manhole (GP).

use super::{Kit, MiniGame, Spec};
use crate::content::Content;
use crate::lcd::Lcd;
use crate::Button;

// ====================================================================== Climb
/// Rocks fall down two vines; switch vines to dodge. Three lives.
pub struct Climb {
    lane: usize,
    rocks: Vec<(usize, f64)>,
    spawn: f64,
    lives: u32,
    scroll: f64,
    score: u32,
}

impl Default for Climb {
    fn default() -> Self {
        Climb { lane: 0, rocks: Vec::new(), spawn: 0.5, lives: 3, scroll: 0.0, score: 0 }
    }
}

const LANES: [i32; 2] = [8, 22];

impl MiniGame for Climb {
    fn spec(&self) -> Spec {
        Spec::new("CLIMB", "A/C: SWITCH VINE", 30.0, 12, 30)
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        let new = match b {
            Button::A => 0,
            Button::C => 1,
            Button::B => 1 - self.lane,
        };
        if new != self.lane {
            self.lane = new;
            k.good();
        }
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        let speed = 9.0 + k.elapsed * 0.45;
        self.scroll += dt * 6.0;
        self.spawn -= dt;
        if self.spawn <= 0.0 {
            self.spawn = (1.1 - k.elapsed * 0.025).max(0.45);
            let lane = k.rng.below(2);
            self.rocks.push((lane, -3.0));
        }
        for r in self.rocks.iter_mut() {
            r.1 += speed * dt;
        }
        let mut keep = Vec::new();
        for &(lane, y) in &self.rocks {
            if lane == self.lane && y + 2.0 >= 10.0 && y <= 14.0 {
                self.lives = self.lives.saturating_sub(1);
                k.hit();
                if self.lives == 0 {
                    k.end();
                }
                continue;
            }
            if y > 16.0 {
                self.score += 1;
                continue;
            }
            keep.push((lane, y));
        }
        self.rocks = keep;
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, t: f64) {
        let off = self.scroll as i32 % 3;
        for vx in LANES {
            for y in -3..16 {
                if (y + off).rem_euclid(3) != 0 {
                    lcd.dot(vx, y);
                }
            }
        }
        for &(lane, y) in &self.rocks {
            lcd.draw(c.prop("rock"), LANES[lane] - 1, y as i32);
        }
        let climber = if (t * 4.0) as i32 % 2 == 0 { "climber1" } else { "climber2" };
        lcd.blit(c.prop(climber), LANES[self.lane] - 2, 10, false, true, None);
        for i in 0..self.lives as i32 {
            lcd.dot(30, 1 + i * 2);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Tug-o-war
/// Mash any button to drag the flag to your side.
pub struct Tug {
    pos: f64,
    lean: f64,
}

impl Default for Tug {
    fn default() -> Self {
        Tug { pos: 0.5, lean: 0.0 }
    }
}

impl MiniGame for Tug {
    fn spec(&self) -> Spec {
        Spec::new("TUG-O-WAR", "MASH B!", 12.0, 11, 20)
    }
    fn press(&mut self, _b: Button, _k: &mut Kit) {
        self.pos -= 0.028;
        self.lean = 0.12;
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        self.pos += (0.05 + k.elapsed * 0.006) * dt + k.rng.uniform(-0.02, 0.03) * dt;
        self.lean = (self.lean - dt).max(0.0);
        self.pos = self.pos.clamp(0.0, 1.0);
        if self.pos <= 0.0 || self.pos >= 1.0 {
            k.end();
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        lcd.hline(5, 26, 9);
        let fx = 5 + (self.pos * 21.0) as i32;
        lcd.vline(fx, 6, 12);
        lcd.dot(fx + 1, 6);
        lcd.dot(fx + 1, 7);
        lcd.vline(15, 13, 15);
        lcd.draw(c.prop("mini_pet"), if self.lean > 0.0 { -1 } else { 0 }, 6);
        lcd.draw(c.prop("mini_rival"), 27, 6);
    }
    fn score(&self) -> u32 {
        ((1.0 - self.pos) * 20.0).round() as u32
    }
}

// ====================================================================== Apples
/// Move the basket; catch apples, dodge bombs.
pub struct Apples {
    bx: f64,
    items: Vec<(i32, f64, bool)>,
    spawn: f64,
    score: u32,
}

impl Default for Apples {
    fn default() -> Self {
        Apples { bx: 12.0, items: Vec::new(), spawn: 0.4, score: 0 }
    }
}

impl MiniGame for Apples {
    fn spec(&self) -> Spec {
        Spec::new("APPLES", "A/C: MOVE BASKET", 25.0, 10, 30)
    }
    fn press(&mut self, b: Button, _k: &mut Kit) {
        match b {
            Button::A => self.bx -= 3.0,
            Button::C => self.bx += 3.0,
            Button::B => {}
        }
        self.bx = self.bx.clamp(0.0, 25.0);
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        if k.held[0] {
            self.bx -= 14.0 * dt;
        }
        if k.held[2] {
            self.bx += 14.0 * dt;
        }
        self.bx = self.bx.clamp(0.0, 25.0);
        self.spawn -= dt;
        if self.spawn <= 0.0 {
            self.spawn = (1.0 - k.elapsed * 0.02).max(0.5);
            let x = k.rng.range(0, 27) as i32;
            let bomb = k.rng.chance(0.28);
            self.items.push((x, -5.0, bomb));
        }
        let fall = 8.0 + k.elapsed * 0.3;
        let mut keep = Vec::new();
        for &(x, y, bomb) in &self.items {
            let y = y + fall * dt;
            let bx = self.bx;
            if y + 4.0 >= 12.0 && y < 14.0 && x as f64 + 4.0 >= bx && x as f64 <= bx + 6.0 {
                if bomb {
                    self.score = self.score.saturating_sub(2);
                    k.hit();
                } else {
                    self.score += 1;
                    k.good();
                }
                continue;
            }
            if y < 16.0 {
                keep.push((x, y, bomb));
            }
        }
        self.items = keep;
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        for &(x, y, bomb) in &self.items {
            lcd.draw(c.prop(if bomb { "bomb" } else { "apple" }), x, y as i32);
        }
        lcd.draw(c.prop("basket"), self.bx as i32, 13);
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Manhole
/// Cover the three manholes (A, B, C) as walkers step over them.
pub struct Manhole {
    lids: [f64; 3],
    walkers: Vec<(f64, f64)>,
    falling: Vec<(i32, f64)>,
    spawn: f64,
    misses: u32,
    score: u32,
}

impl Default for Manhole {
    fn default() -> Self {
        Manhole { lids: [0.0; 3], walkers: Vec::new(), falling: Vec::new(), spawn: 0.3, misses: 0, score: 0 }
    }
}

const HOLES: [i32; 3] = [7, 16, 25];

impl MiniGame for Manhole {
    fn spec(&self) -> Spec {
        let mut s = Spec::new("MANHOLE", "A B C: COVER HOLE", 30.0, 8, 25);
        s.gp_per_point = 10;
        s
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        self.lids[b.index()] = 0.5;
        k.good();
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        for l in self.lids.iter_mut() {
            *l = (*l - dt).max(0.0);
        }
        self.spawn -= dt;
        if self.spawn <= 0.0 {
            self.spawn = (2.0 - k.elapsed * 0.04).max(0.9);
            let speed = k.rng.uniform(6.0, 8.0 + k.elapsed * 0.15);
            self.walkers.push((-3.0, speed));
        }
        let mut keep = Vec::new();
        for &(x, sp) in &self.walkers {
            let x = x + sp * dt;
            let cx = x + 1.0;
            let fell = HOLES.iter().enumerate().find(|(i, &hx)| (cx - hx as f64).abs() < 0.8 && self.lids[*i] <= 0.0);
            if let Some((_, &hx)) = fell {
                self.falling.push((hx - 1, 8.0));
                self.misses += 1;
                k.hit();
                if self.misses >= 3 {
                    k.end();
                }
                continue;
            }
            if x > 32.0 {
                self.score += 1;
                k.good();
                continue;
            }
            keep.push((x, sp));
        }
        self.walkers = keep;
        self.falling = self.falling.iter().filter(|f| f.1 < 16.0).map(|&(x, y)| (x, y + 20.0 * dt)).collect();
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        for x in 0..32 {
            if !HOLES.iter().any(|&hx| (x - hx).abs() <= 2) {
                lcd.dot(x, 13);
            }
        }
        for (i, &hx) in HOLES.iter().enumerate() {
            if self.lids[i] > 0.0 {
                lcd.hline(hx - 2, hx + 2, 13);
            } else {
                lcd.dot(hx - 2, 14);
                lcd.dot(hx + 2, 14);
            }
        }
        for &(x, _) in &self.walkers {
            let w = if (x as i32) % 2 == 0 { "walker1" } else { "walker2" };
            lcd.draw(c.prop(w), x as i32, 8);
        }
        for &(x, y) in &self.falling {
            lcd.draw(c.prop("faller"), x, y as i32);
        }
        for i in 0..self.misses as i32 {
            lcd.dot(31 - i * 2, 0);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}
