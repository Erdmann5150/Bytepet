//! Quick sums for school and job interviews: A picks the left answer, C the right one.

use super::{Kit, MiniGame, Spec};
use crate::content::Content;
use crate::lcd::Lcd;
use crate::{Button, Tune};

pub struct Quiz {
    title: &'static str,
    rounds: u32,
    n: u32,
    score: u32,
    q: String,
    opts: (i64, i64),
    right_is_correct: bool,
    qt: f64,
    fb: f64,
    fb_ok: bool,
    started: bool,
}

const ANSWER_TIME: f64 = 6.0;

impl Quiz {
    pub fn new(title: &'static str, rounds: u32) -> Quiz {
        Quiz {
            title,
            rounds,
            n: 0,
            score: 0,
            q: String::new(),
            opts: (0, 0),
            right_is_correct: false,
            qt: ANSWER_TIME,
            fb: 0.0,
            fb_ok: false,
            started: false,
        }
    }

    fn new_question(&mut self, k: &mut Kit) {
        let a = k.rng.range(1, 9);
        let b = k.rng.range(1, 9);
        let (q, ans) = if k.rng.chance(0.5) {
            (format!("{a}+{b}"), a + b)
        } else {
            let (hi, lo) = (a.max(b), a.min(b));
            (format!("{hi}-{lo}"), hi - lo)
        };
        let mut wrong = ans + [-3, -2, -1, 1, 2, 3][k.rng.below(6)];
        if wrong < 0 {
            wrong = ans + 2;
        }
        self.right_is_correct = k.rng.chance(0.5);
        self.opts = if self.right_is_correct { (wrong, ans) } else { (ans, wrong) };
        self.q = q;
        self.qt = ANSWER_TIME;
    }

    fn answer(&mut self, ok: bool, k: &mut Kit) {
        self.fb = 0.6;
        self.fb_ok = ok;
        if ok {
            self.score += 1;
            k.play(Tune::Ok);
        } else {
            k.play(Tune::No);
        }
    }
}

impl MiniGame for Quiz {
    fn spec(&self) -> Spec {
        Spec::new(self.title, "A=LEFT C=RIGHT", 999.0, self.rounds.div_ceil(2), self.rounds)
    }
    fn start(&mut self, k: &mut Kit) {
        self.started = true;
        self.new_question(k);
    }
    fn press(&mut self, b: Button, k: &mut Kit) {
        if self.fb > 0.0 || b == Button::B {
            return;
        }
        let picked_right = b == Button::C;
        self.answer(picked_right == self.right_is_correct, k);
    }
    fn tick(&mut self, dt: f64, k: &mut Kit) {
        if self.fb > 0.0 {
            self.fb -= dt;
            if self.fb <= 0.0 {
                self.n += 1;
                if self.n >= self.rounds {
                    k.end();
                } else {
                    self.new_question(k);
                }
            }
            return;
        }
        self.qt -= dt;
        if self.qt <= 0.0 {
            self.answer(false, k);
        }
    }
    fn draw(&self, lcd: &mut Lcd, c: &Content, _pet: &str, _t: f64) {
        if self.fb > 0.0 {
            lcd.draw(c.prop(if self.fb_ok { "check" } else { "cross" }), 13, 5);
            return;
        }
        lcd.text_center(&format!("{}=?", self.q), 1);
        lcd.text(&self.opts.0.to_string(), 1, 9);
        let r = self.opts.1.to_string();
        lcd.text(&r, 32 - r.len() as i32 * 4, 9);
        lcd.hline(0, (31.0 * self.qt / ANSWER_TIME) as i32, 15);
    }
    fn score(&self) -> u32 {
        self.score
    }
}
