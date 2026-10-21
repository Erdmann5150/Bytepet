//! The main screen: the pet wandering around, the icon cursor, and the reactions to life
//! events (hatching, evolving, leaving).

use crate::content::{Pose, Stage};
use crate::lcd::Lcd;
use crate::sim::{Event, Medicine};
use crate::{Button, Tune};

use super::care::{
    Clock, DisciplineMenu, Evolve, Flush, FoodMenu, GameMenu, Gone, Hatch, MedicineAnim, React, ReactKind, Status,
    POOP_POS,
};
use super::town::TownMenu;
use super::{back, boxed, msg, Ctx, DrawCx, NameEntry, Res, Screen, Tr};

/// Icons that do one thing and are done (clean, lights, medicine), rather than open a menu.
pub const ONE_SHOT: [usize; 3] = [2, 4, 5];

pub struct MainScreen {
    x: f64,
    flip: bool,
    move_t: f64,
    call_beep: f64,
    t: f64,
}

impl MainScreen {
    pub fn new() -> MainScreen {
        MainScreen { x: 8.0, flip: false, move_t: 0.0, call_beep: 0.0, t: 0.0 }
    }

    fn max_x(cx_poop: u8, sick: bool) -> f64 {
        let mut right = 32;
        if cx_poop >= 1 || sick {
            right = 26;
        }
        if cx_poop >= 3 {
            right = 20;
        }
        (right - 16).max(0) as f64
    }

    /// Activate icon `i` (0..9).
    pub fn activate(&mut self, cx: &mut Ctx, i: usize) -> Tr {
        cx.play(Tune::Click);
        let stage = cx.game.pet.stage;
        if stage == Stage::Gone {
            return Tr::Push(boxed(Gone::new(cx.game.album.last().map(|a| a.character.clone()).unwrap_or_default())));
        }
        let egg_ok = [0, 8];
        let sleep_ok = [0, 2, 4, 5, 8];
        if stage == Stage::Egg && !egg_ok.contains(&i) {
            cx.play(Tune::No);
            return Tr::Push(msg(&[("...", "")]));
        }
        if cx.game.pet.asleep && !sleep_ok.contains(&i) {
            cx.play(Tune::No);
            return Tr::Push(msg(&[("ZZZ...", "SLEEPING")]));
        }
        match i {
            0 => Tr::Push(boxed(Status::new())),
            1 => Tr::Push(boxed(FoodMenu::new())),
            2 => {
                let n = cx.game.clean();
                Tr::Push(boxed(Flush::new(n)))
            }
            3 => {
                if stage == Stage::Baby {
                    cx.play(Tune::No);
                    Tr::Push(msg(&[("TOO", "YOUNG")]))
                } else {
                    Tr::Push(boxed(GameMenu::new()))
                }
            }
            4 => {
                cx.game.toggle_lights();
                cx.play(Tune::Ok);
                Tr::None
            }
            5 => match cx.game.medicine() {
                Medicine::NotSick => Tr::Push(boxed(React::new(ReactKind::No))),
                Medicine::Cured => Tr::PushAll(vec![msg(&[("CURED!", "")]), boxed(MedicineAnim::new())]),
                Medicine::Again => Tr::PushAll(vec![msg(&[("ONE MORE", "DOSE")]), boxed(MedicineAnim::new())]),
            },
            6 => Tr::Push(boxed(DisciplineMenu::new())),
            7 => {
                if matches!(stage, Stage::Egg | Stage::Baby) {
                    cx.play(Tune::No);
                    Tr::Push(msg(&[("TOO", "YOUNG")]))
                } else {
                    Tr::Push(boxed(crate::link::LinkScreen::new()))
                }
            }
            8 => Tr::Push(boxed(TownMenu::new())),
            _ => Tr::None,
        }
    }
}

impl Default for MainScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for MainScreen {
    fn name(&self) -> &'static str {
        "Main"
    }
    fn is_main(&self) -> bool {
        true
    }
    fn animate(&mut self, dt: f64) {
        self.t += dt;
    }
    fn activate_icon(&mut self, cx: &mut Ctx, i: usize) -> Tr {
        self.activate(cx, i)
    }

    fn prev(&mut self, cx: &mut Ctx) -> Option<Tr> {
        cx.play(Tune::Click);
        // from no selection, back lands on the last icon
        cx.ui.sel = back(cx.ui.sel.max(0) as usize, 9) as i32;
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::A => {
                cx.play(Tune::Click);
                cx.ui.sel = (cx.ui.sel + 1).rem_euclid(9);
                Tr::None
            }
            Button::C => {
                cx.play(Tune::Click);
                cx.ui.sel = -1;
                self.x = 8.0;
                Tr::None
            }
            Button::B => {
                if cx.ui.sel < 0 {
                    cx.play(Tune::Click);
                    Tr::Push(boxed(Clock::new()))
                } else {
                    let i = cx.ui.sel as usize;
                    self.activate(cx, i)
                }
            }
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        let pet_id = cx.game.pet.id;
        if cx.game.pet.name.is_empty()
            && matches!(cx.game.pet.stage, Stage::Egg | Stage::Baby)
            && cx.ui.asked_name_for != pet_id
        {
            cx.ui.asked_name_for = pet_id;
            return Tr::Push(boxed(NameEntry::new()));
        }
        if let Some(ev) = cx.pending.pop_front() {
            match ev {
                Event::Hatch => return Tr::Push(boxed(Hatch::new())),
                Event::Evolve(ch) => return Tr::Push(boxed(Evolve::new(ch))),
                Event::Gone => {
                    let ch = cx.game.album.last().map(|a| a.character.clone()).unwrap_or_default();
                    return Tr::Push(boxed(Gone::new(ch)));
                }
                Event::Call(_) => {
                    cx.play(Tune::Call);
                    self.call_beep = 0.0;
                }
                Event::Mail => cx.play(Tune::Mail),
                Event::Sick => cx.play(Tune::No),
                Event::Show => cx.play(Tune::Ok),
                _ => {}
            }
            return Tr::None;
        }
        if cx.game.pet.call.is_some() {
            self.call_beep += dt;
            if self.call_beep >= 60.0 {
                self.call_beep = 0.0;
                cx.play(Tune::Call);
            }
        }
        // wander; poop or sickness can corner the pet at any time, and it stays there once
        // they're gone rather than jumping back
        let hi = MainScreen::max_x(cx.game.pet.poop, cx.game.pet.sick);
        self.x = self.x.min(hi);
        self.move_t += dt;
        if self.move_t >= 0.7 && cx.game.pet.awake() {
            self.move_t = 0.0;
            let r = cx.game.rng.f64();
            if r < 0.25 {
                self.flip = !self.flip;
            } else if r < 0.85 {
                self.x += if self.flip { -2.0 } else { 2.0 };
            }
            if self.x <= 0.0 {
                self.x = 0.0;
                self.flip = false;
            }
            if self.x >= hi {
                self.x = hi;
                self.flip = true;
            }
        }
        Tr::None
    }

    fn result(&mut self, cx: &mut Ctx, r: Res) -> Tr {
        if let Res::Name(n) = r {
            cx.game.pet.name = n;
        }
        Tr::None
    }

    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let p = &cx.game.pet;
        let c = cx.content;
        if p.stage == Stage::Gone {
            return;
        }
        if !p.lights {
            lcd.clear(1);
            return;
        }
        let f = (self.t * 2.0) as i32 % 2;
        if p.stage == Stage::Egg {
            let wobble = if p.stage_s > 30.0 { f } else { 0 };
            lcd.draw(c.prop("egg"), 8 + wobble, 0);
            return;
        }
        for &(x, y) in POOP_POS.iter().take(p.poop.min(4) as usize) {
            lcd.draw(c.prop(if f == 0 { "poop1" } else { "poop2" }), x, y);
        }
        if p.sick {
            lcd.draw(c.prop("skull"), 27, 0);
        }
        let x = self.x.min(MainScreen::max_x(p.poop, p.sick)) as i32;
        if p.asleep {
            lcd.draw(c.sprite(&p.character, Pose::Sleep), x, 0);
            let zx = (18 + (self.t as i32 % 3) * 2).min(28);
            lcd.draw(c.prop("zzz"), zx, 1 + f);
            return;
        }
        if p.praise_t > 0.0 {
            // showing off: dance with a note (praise now!)
            lcd.draw(c.prop("note"), if x < 12 { 28 } else { 0 }, 1 + f);
            lcd.blit(c.sprite(&p.character, Pose::Happy), x, -f, f == 1, false, None);
            return;
        }
        let pose = if p.sick || p.call.is_some() {
            Pose::Sad
        } else if f == 1 {
            Pose::Idle2
        } else {
            Pose::Idle1
        };
        lcd.blit(c.sprite(&p.character, pose), x, 0, self.flip, false, None);
    }
}
