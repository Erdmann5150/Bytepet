//! Work games, one per job.

use super::{Kit, MiniGame, Spec};
use crate::content::Content;
use crate::lcd::Lcd;
use crate::{Button, Tune};

// ====================================================================== Office
/// A paper lands on one of three desks: press its button before time runs out.
pub struct Office {
    slot: Option<usize>,
    wait: f64,
    deadline: f64,
    misses: u32,
    score: u32,
}

impl Default for Office {
    fn default() -> Self {
        Office { slot: None, wait: 0.4, deadline: 0.0, misses: 0, score: 0 }
    }
}

const DESKS: [i32; 3] = [3, 14, 25];

impl Office {
    fn miss(&mut self, k: &mut Kit) {
        self.misses += 1;
        k.hit();
        if self.misses >= 3 {
            k.end();
        }
    }
}

impl MiniGame for Office {
    fn spec(&self) -> Spec {
        Spec::new("OFFICE", "A B C: GRAB PAPERS", 25.0, 10, 25)
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        let Some(slot) = self.slot else { return };
        if b.index() == slot {
            self.score += 1;
            k.good();
        } else {
            self.miss(k);
        }
        self.slot = None;
        self.wait = 0.25;
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        match self.slot {
            None => {
                self.wait -= dt;
                if self.wait <= 0.0 {
                    self.slot = Some(k.rng.below(3));
                    self.deadline = (1.6 - k.elapsed * 0.035).max(0.7);
                }
            }
            Some(_) => {
                self.deadline -= dt;
                if self.deadline <= 0.0 {
                    self.slot = None;
                    self.wait = 0.3;
                    self.miss(k);
                }
            }
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        for &x in &DESKS {
            lcd.hline(x - 2, x + 6, 10);
            lcd.vline(x - 1, 11, 13);
            lcd.vline(x + 5, 11, 13);
        }
        if let Some(s) = self.slot {
            lcd.draw(c.prop("doc"), DESKS[s], 4);
        }
        for i in 0..self.misses as i32 {
            lcd.dot(31 - i * 2, 0);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Chef
/// Copy the three-ingredient order: bun = A, meat = B, leaf = C.
#[derive(Default)]
pub struct Chef {
    order: [usize; 3],
    k: usize,
    score: u32,
}

impl Chef {
    fn new_order(&mut self, k: &mut Kit) {
        for o in self.order.iter_mut() {
            *o = k.rng.below(3);
        }
        self.k = 0;
    }
}

const INGREDIENTS: [&str; 3] = ["bun", "meat", "leaf"];

impl MiniGame for Chef {
    fn spec(&self) -> Spec {
        Spec::new("CHEF", "COPY THE ORDER A B C", 30.0, 5, 10)
    }
    fn start(&mut self, k: &mut Kit) {
        self.new_order(k);
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        if b.index() == self.order[self.k] {
            self.k += 1;
            k.good();
            if self.k == 3 {
                self.score += 1;
                k.play(Tune::Ok);
                self.new_order(k);
            }
        } else {
            k.hit();
            self.k = 0;
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        for (i, &ing) in self.order.iter().enumerate() {
            let x = 2 + i as i32 * 7;
            lcd.draw(c.prop(INGREDIENTS[ing]), x, 1);
            if i < self.k {
                lcd.hline(x, x + 4, 5);
            }
        }
        lcd.text("ABC", 21, 1);
        lcd.draw(c.prop("bun"), 13, 13);
        let mut y = 13;
        for i in 0..self.k {
            y -= 3;
            lcd.draw(c.prop(INGREDIENTS[self.order[i]]), 13, y);
        }
        lcd.text(&self.score.to_string(), 26, 10);
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Firefighter
/// Move the hose (A/C) and spray (B) before any fire reaches the top of its window.
pub struct Fire {
    fires: [f64; 3],
    hose: usize,
    spray: f64,
    spawn: f64,
    misses: u32,
    score: u32,
}

impl Default for Fire {
    fn default() -> Self {
        Fire { fires: [0.0; 3], hose: 1, spray: 0.0, spawn: 0.5, misses: 0, score: 0 }
    }
}

const WINDOWS: [i32; 3] = [2, 13, 24];

impl MiniGame for Fire {
    fn spec(&self) -> Spec {
        Spec::new("FIREFIGHTER", "A/C: MOVE  B: SPRAY", 30.0, 8, 15)
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        match b {
            Button::A => self.hose = self.hose.saturating_sub(1),
            Button::C => self.hose = (self.hose + 1).min(2),
            Button::B => {
                self.spray = 0.25;
                let f = self.fires[self.hose];
                if f > 0.0 {
                    self.fires[self.hose] = (f - 1.6).max(0.0);
                    if self.fires[self.hose] == 0.0 {
                        self.score += 1;
                        k.play(Tune::Ok);
                    }
                }
            }
        }
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        self.spray = (self.spray - dt).max(0.0);
        self.spawn -= dt;
        if self.spawn <= 0.0 {
            self.spawn = (2.0 - k.elapsed * 0.04).max(0.8);
            let i = k.rng.below(3);
            if self.fires[i] == 0.0 {
                self.fires[i] = 0.5;
            }
        }
        for i in 0..3 {
            if self.fires[i] > 0.0 {
                self.fires[i] += dt * (0.8 + k.elapsed * 0.02);
                if self.fires[i] >= 4.0 {
                    self.fires[i] = 0.0;
                    self.misses += 1;
                    k.hit();
                    if self.misses >= 3 {
                        k.end();
                    }
                }
            }
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        for (i, &x) in WINDOWS.iter().enumerate() {
            lcd.draw(c.prop("window"), x, 1);
            if self.fires[i] > 0.0 {
                let h = self.fires[i] as i32;
                for k in 0..=h {
                    let w = (2 - k / 2).max(0);
                    lcd.hline(x + 2 - w, x + 2 + w, 5 - k);
                }
            }
        }
        let hx = WINDOWS[self.hose] + 1;
        lcd.draw(c.prop("hose"), hx, 13);
        if self.spray > 0.0 {
            for y in (7..13).step_by(2) {
                lcd.dot(hx + 1, y);
            }
        }
        for i in 0..self.misses as i32 {
            lcd.dot(31 - i * 2, 15);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Rock star
/// Notes scroll left in three rows; hit A/B/C (top/middle/bottom) as they cross the line.
pub struct Rock {
    notes: Vec<(usize, f64)>,
    to_spawn: u32,
    spawn: f64,
    score: u32,
}

const NOTES: u32 = 24;
const ROWS: [i32; 3] = [1, 6, 11];

impl Default for Rock {
    fn default() -> Self {
        Rock { notes: Vec::new(), to_spawn: NOTES, spawn: 0.3, score: 0 }
    }
}

impl MiniGame for Rock {
    fn spec(&self) -> Spec {
        Spec::new("ROCK STAR", "A B C: HIT THE NOTES", 40.0, 12, NOTES)
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        let lane = b.index();
        if let Some(pos) = self.notes.iter().position(|&(l, x)| l == lane && (1.0..=9.0).contains(&x)) {
            self.notes.remove(pos);
            self.score += 1;
            k.good();
        } else {
            k.hit();
        }
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        self.spawn -= dt;
        if self.spawn <= 0.0 && self.to_spawn > 0 {
            self.spawn = k.rng.uniform(0.55, 1.1);
            self.to_spawn -= 1;
            let lane = k.rng.below(3);
            self.notes.push((lane, 32.0));
        }
        let speed = 13.0 + k.elapsed * 0.15;
        for n in self.notes.iter_mut() {
            n.1 -= speed * dt;
        }
        self.notes.retain(|n| n.1 >= -3.0);
        if self.to_spawn == 0 && self.notes.is_empty() {
            k.end();
        }
    }
    fn draw(&self, lcd: &mut Lcd, _c: &Content, _pet: &str, _t: f64) {
        for y in (0..16).step_by(2) {
            lcd.dot(5, y);
        }
        for &(lane, x) in &self.notes {
            let y = ROWS[lane];
            lcd.rect(x as i32, y, 3, 3, true);
            lcd.dot(x as i32 + 3, y - 1);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}

// ====================================================================== Scientist
/// Watch the flasks light up, then repeat the order. Up to six rounds.
pub struct Science {
    seq: Vec<usize>,
    showing: bool,
    k: usize,
    pt: f64,
    lit: Option<usize>,
    lit_t: f64,
    score: u32,
}

impl Default for Science {
    fn default() -> Self {
        Science { seq: Vec::new(), showing: true, k: 0, pt: -0.4, lit: None, lit_t: 0.0, score: 0 }
    }
}

const FLASKS: [i32; 3] = [3, 14, 25];
const ROUNDS: u32 = 6;

impl Science {
    fn show(&mut self) {
        self.showing = true;
        self.k = 0;
        self.pt = -0.4;
        self.lit = None;
    }
}

impl MiniGame for Science {
    fn spec(&self) -> Spec {
        Spec::new("SCIENTIST", "REPEAT THE FLASKS", 999.0, 3, ROUNDS)
    }
    fn start(&mut self, k: &mut Kit) {
        self.seq = (0..3).map(|_| k.rng.below(3)).collect();
        self.show();
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        if self.showing {
            return;
        }
        let i = b.index();
        self.lit = Some(i);
        self.lit_t = 0.25;
        if i == self.seq[self.k] {
            self.k += 1;
            k.good();
            if self.k == self.seq.len() {
                self.score += 1;
                k.play(Tune::Ok);
                if self.score >= ROUNDS {
                    k.end();
                    return;
                }
                let next = k.rng.below(3);
                self.seq.push(next);
                self.show();
            }
        } else {
            k.hit();
            k.end();
        }
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        if self.showing {
            self.pt += dt;
            let step = 0.65;
            if self.pt < 0.0 {
                self.lit = None;
                return;
            }
            let idx = (self.pt / step) as usize;
            if idx >= self.seq.len() {
                self.showing = false;
                self.k = 0;
                self.lit = None;
                return;
            }
            let new = if self.pt % step < 0.45 { Some(self.seq[idx]) } else { None };
            if new.is_some() && self.lit.is_none() {
                k.good();
            }
            self.lit = new;
        } else if self.lit.is_some() {
            self.lit_t -= dt;
            if self.lit_t <= 0.0 {
                self.lit = None;
            }
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, t: f64) {
        for (i, &x) in FLASKS.iter().enumerate() {
            lcd.draw(c.prop(if self.lit == Some(i) { "flask" } else { "flask_empty" }), x, 5);
        }
        if !self.showing && (t * 2.0) as i32 % 2 == 1 {
            lcd.text("GO", 0, 11);
        }
        for i in 0..self.score as i32 {
            lcd.dot(26 + i, 15);
        }
    }
    fn score(&self) -> u32 {
        self.score
    }
}
