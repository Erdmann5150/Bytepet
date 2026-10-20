//! Helpers for driving a headless Core in tests.
#![allow(dead_code)]

use bytepet::content::{Content, Stage};
use bytepet::core::Core;
use bytepet::rng::Pcg32;
use bytepet::sim::Game;
use bytepet::Button;

pub struct Rig {
    pub core: Core,
    pub wall: f64,
}

impl Rig {
    pub fn new(seed: u64) -> Rig {
        let content = Content::builtin();
        let game = Game::new(&content, Pcg32::new(seed));
        let mut core = Core::new(content, game);
        core.hour_at = |_| 12.0; // always daytime
        Rig { core, wall: 1_800_000_000.0 }
    }

    /// A named pet already at `stage`.
    pub fn with_pet(seed: u64, stage: Stage, character: &str) -> Rig {
        let mut r = Rig::new(seed);
        let p = &mut r.core.game.pet;
        p.name = "TEST".into();
        p.stage = stage;
        p.character = character.into();
        p.hunger = 4;
        p.happy = 4;
        p.t_poop = 1e9;
        p.t_mail = 1e9;
        p.t_fake = 1e9;
        p.t_show = 1e9;
        p.t_hunger = 1e9;
        p.t_happy = 1e9;
        r.core.ui.asked_name_for = p.id;
        r
    }

    pub fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.wall += 1.0 / 30.0;
            self.core.frame_at(1.0 / 30.0, self.wall);
        }
    }

    pub fn tap(&mut self, b: Button) {
        self.core.press(b);
        self.frames(1);
        self.core.release(b);
        self.frames(1);
    }

    pub fn taps(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                'A' => self.tap(Button::A),
                'B' => self.tap(Button::B),
                'C' => self.tap(Button::C),
                _ => self.frames(10),
            }
        }
    }

    pub fn top(&self) -> &'static str {
        self.core.top_name()
    }

    /// Press B until the top screen is `name` (or give up).
    pub fn until(&mut self, name: &str, max: usize) -> bool {
        for _ in 0..max {
            if self.top() == name {
                return true;
            }
            self.frames(1);
        }
        self.top() == name
    }

    pub fn back_to_main(&mut self) {
        for _ in 0..40 {
            if self.top() == "Main" {
                return;
            }
            self.tap(Button::C);
            self.frames(5);
        }
    }
}
