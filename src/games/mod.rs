//! Mini-games. Each game implements `MiniGame`; `GameHost` wraps it as a screen with an intro,
//! a timer and a score card, and pops with `Res::Score(n)` (or `Res::Cancel` from the intro).
//! Games draw their randomness from the host's seeded generator, so two linked devices given
//! the same seed play exactly the same round.

use crate::content::{Content, JobGame, Skill};
use crate::lcd::Lcd;
use crate::rng::Pcg32;
use crate::screens::{Ctx, DrawCx, Res, Screen, Tr};
use crate::{Button, Tune};

mod jobs;
mod quiz;
mod solo;
mod versus;

pub use jobs::{Chef, Fire, Office, Rock, Science};
pub use quiz::Quiz;
pub use solo::{Apples, Climb, Manhole, Tug};
pub use versus::{Bump, Eats, Flags};

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub title: &'static str,
    pub howto: &'static str,
    /// Seconds before time runs out (games that end on their own use a large number).
    pub duration: f64,
    pub win_score: u32,
    pub max_score: u32,
    pub gp_per_point: u32,
}

impl Spec {
    pub const fn new(title: &'static str, howto: &'static str, duration: f64, win: u32, max: u32) -> Spec {
        Spec { title, howto, duration, win_score: win, max_score: max, gp_per_point: 0 }
    }
}

/// What a game can do while running.
pub struct Kit<'a> {
    pub rng: &'a mut Pcg32,
    pub sfx: &'a mut Vec<Tune>,
    pub held: [bool; 3],
    /// Seconds since play started.
    pub elapsed: f64,
    flash: &'a mut f64,
    ended: &'a mut bool,
}

impl Kit<'_> {
    /// A mistake: flash the screen and buzz.
    pub fn hit(&mut self) {
        *self.flash = 0.4;
        self.sfx.push(Tune::Hit);
    }
    pub fn good(&mut self) {
        self.sfx.push(Tune::Tick);
    }
    pub fn play(&mut self, t: Tune) {
        self.sfx.push(t);
    }
    pub fn end(&mut self) {
        *self.ended = true;
    }
}

pub trait MiniGame {
    fn spec(&self) -> Spec;
    fn start(&mut self, _k: &mut Kit) {}
    fn press(&mut self, _b: Button, _k: &mut Kit) {}
    fn release(&mut self, _b: Button, _k: &mut Kit) {}
    fn tick(&mut self, _dt: f64, _k: &mut Kit) {}
    fn draw(&self, lcd: &mut Lcd, c: &Content, pet_char: &str, t: f64);
    fn score(&self) -> u32;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Intro,
    Play,
    End,
}

pub struct GameHost {
    game: Box<dyn MiniGame>,
    rng: Pcg32,
    state: State,
    st: f64,
    t: f64,
    left: f64,
    flash: f64,
    ended: bool,
}

impl GameHost {
    pub fn new(game: Box<dyn MiniGame>, seed: u64) -> GameHost {
        let left = game.spec().duration;
        GameHost { game, rng: Pcg32::new(seed), state: State::Intro, st: 0.0, t: 0.0, left, flash: 0.0, ended: false }
    }

    fn kit<'a>(
        rng: &'a mut Pcg32,
        sfx: &'a mut Vec<Tune>,
        held: [bool; 3],
        elapsed: f64,
        flash: &'a mut f64,
        ended: &'a mut bool,
    ) -> Kit<'a> {
        Kit { rng, sfx, held, elapsed, flash, ended }
    }

    fn begin(&mut self, cx: &mut Ctx) {
        self.state = State::Play;
        self.st = 0.0;
        cx.play(Tune::Ok);
        let mut k = GameHost::kit(&mut self.rng, cx.sfx, cx.held, 0.0, &mut self.flash, &mut self.ended);
        self.game.start(&mut k);
    }

    fn finish_play(&mut self, cx: &mut Ctx) {
        self.state = State::End;
        self.st = 0.0;
        cx.play(Tune::Tick);
    }

    /// For tests and the link screen: the game's current score.
    pub fn score(&self) -> u32 {
        self.game.score()
    }
}

impl Screen for GameHost {
    fn name(&self) -> &'static str {
        "Game"
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match self.state {
            State::Intro => {
                if b == Button::B && self.st > 0.3 {
                    self.begin(cx);
                } else if b == Button::C {
                    return Tr::PopWith(Res::Cancel);
                }
            }
            State::Play => {
                let elapsed = self.game.spec().duration - self.left;
                let mut k = GameHost::kit(&mut self.rng, cx.sfx, cx.held, elapsed, &mut self.flash, &mut self.ended);
                self.game.press(b, &mut k);
                if self.ended {
                    self.finish_play(cx);
                }
            }
            State::End => {
                if self.st > 0.5 {
                    return Tr::PopWith(Res::Score(self.game.score()));
                }
            }
        }
        Tr::None
    }
    fn release(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        if self.state == State::Play {
            let elapsed = self.game.spec().duration - self.left;
            let mut k = GameHost::kit(&mut self.rng, cx.sfx, cx.held, elapsed, &mut self.flash, &mut self.ended);
            self.game.release(b, &mut k);
        }
        Tr::None
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        self.st += dt;
        self.flash = (self.flash - dt).max(0.0);
        match self.state {
            State::Intro => {
                if self.st >= 2.4 {
                    self.begin(cx);
                }
            }
            State::Play => {
                self.left -= dt;
                let elapsed = self.game.spec().duration - self.left;
                let mut k = GameHost::kit(&mut self.rng, cx.sfx, cx.held, elapsed, &mut self.flash, &mut self.ended);
                self.game.tick(dt, &mut k);
                if self.ended || self.left <= 0.0 {
                    self.finish_play(cx);
                }
            }
            State::End => {
                if self.st >= 1.6 {
                    return Tr::PopWith(Res::Score(self.game.score()));
                }
            }
        }
        Tr::None
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let spec = self.game.spec();
        match self.state {
            State::Intro => {
                lcd.line(spec.title, 1, self.st, true);
                lcd.line(if spec.howto.is_empty() { "B:START" } else { spec.howto }, 9, self.st, true);
            }
            State::End => {
                lcd.text_center("SCORE", 1);
                lcd.text_center(&self.game.score().to_string(), 9);
            }
            State::Play => {
                self.game.draw(lcd, cx.content, &cx.game.pet.character, self.t);
                if self.flash > 0.0 && (self.flash * 12.0) as i32 % 2 == 1 {
                    lcd.invert();
                }
            }
        }
    }
}

// ====================================================================== registries
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoloGame {
    Climb,
    Tug,
    Apples,
    Manhole,
}

impl SoloGame {
    pub const ALL: [SoloGame; 4] = [SoloGame::Climb, SoloGame::Tug, SoloGame::Apples, SoloGame::Manhole];
    pub fn label(self) -> &'static str {
        match self {
            SoloGame::Climb => "CLIMB",
            SoloGame::Tug => "TUG",
            SoloGame::Apples => "APPLES",
            SoloGame::Manhole => "MANHOLE",
        }
    }
    pub fn skill(self) -> Option<Skill> {
        match self {
            SoloGame::Climb => Some(Skill::Humor),
            SoloGame::Tug => Some(Skill::Gorgeous),
            SoloGame::Apples => Some(Skill::Passion),
            SoloGame::Manhole => None,
        }
    }
    pub fn make(self) -> Box<dyn MiniGame> {
        match self {
            SoloGame::Climb => Box::new(Climb::default()),
            SoloGame::Tug => Box::new(Tug::default()),
            SoloGame::Apples => Box::new(Apples::default()),
            SoloGame::Manhole => Box::new(Manhole::default()),
        }
    }
}

pub fn job_game(g: JobGame) -> Box<dyn MiniGame> {
    match g {
        JobGame::Office => Box::new(Office::default()),
        JobGame::Science => Box::new(Science::default()),
        JobGame::Chef => Box::new(Chef::default()),
        JobGame::Fire => Box::new(Fire::default()),
        JobGame::Rock => Box::new(Rock::default()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersusGame {
    Eats,
    Flags,
    Bump,
}

impl VersusGame {
    pub const ALL: [VersusGame; 3] = [VersusGame::Eats, VersusGame::Flags, VersusGame::Bump];
    pub fn id(self) -> &'static str {
        match self {
            VersusGame::Eats => "EATS",
            VersusGame::Flags => "FLAGS",
            VersusGame::Bump => "BUMP",
        }
    }
    pub fn from_id(s: &str) -> Option<VersusGame> {
        VersusGame::ALL.iter().copied().find(|g| g.id() == s)
    }
    pub fn make(self) -> Box<dyn MiniGame> {
        match self {
            VersusGame::Eats => Box::new(Eats::default()),
            VersusGame::Flags => Box::new(Flags::default()),
            VersusGame::Bump => Box::new(Bump::default()),
        }
    }
}
