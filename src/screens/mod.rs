//! LCD screens. Each screen handles input and returns a `Tr` (transition) instead of calling
//! back into other screens. A child that finishes pops itself with a `Res`, and the screen
//! underneath it receives that result in `result()`. Flows that span several screens keep
//! their own small state machine (see `town.rs`), and capture the pet id when they start so a
//! result that arrives after the pet has left is ignored.

use std::collections::VecDeque;

use crate::content::Content;
use crate::lcd::Lcd;
use crate::sim::{Event, Game};
use crate::{Button, Tune};

pub mod care;
pub mod main_screen;
pub mod town;

pub use main_screen::MainScreen;

/// Mutable access a screen gets while handling input or updating.
pub struct Ctx<'a> {
    pub game: &'a mut Game,
    pub content: &'a Content,
    pub ui: &'a mut UiState,
    pub sfx: &'a mut Vec<Tune>,
    pub held: [bool; 3],
    /// Simulation events waiting for the main screen (animations, beeps).
    pub pending: &'a mut VecDeque<Event>,
}

impl Ctx<'_> {
    pub fn play(&mut self, t: Tune) {
        self.sfx.push(t);
    }
    pub fn c(&self) -> &Content {
        self.content
    }
    /// Is this still the pet that started a flow, and is it alive?
    pub fn same_pet(&self, id: u64) -> bool {
        self.game.pet.id == id && self.game.pet.alive()
    }
}

/// Read-only view for drawing.
pub struct DrawCx<'a> {
    pub game: &'a Game,
    pub content: &'a Content,
    pub ui: &'a UiState,
}

/// UI state shared with the device renderer and the desktop shell.
#[derive(Default, Debug, Clone)]
pub struct UiState {
    /// Highlighted icon on the main screen (-1 = none).
    pub sel: i32,
    pub show_help: bool,
    pub asked_name_for: u64,
    /// Requests for the desktop shell.
    pub request: Option<Request>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Open the desktop settings menu (from Town > SETUP).
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Res {
    Score(u32),
    Pick(usize),
    Yes(bool),
    Name(String),
    Done,
    Cancel,
}

pub enum Tr {
    None,
    Push(Box<dyn Screen>),
    /// Push several; the last one ends up on top (so it is seen first).
    PushAll(Vec<Box<dyn Screen>>),
    Pop,
    PopWith(Res),
    Replace(Box<dyn Screen>),
    ToMain,
    ToMainThen(Vec<Box<dyn Screen>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Backspace,
    Enter,
}

pub trait Screen {
    fn press(&mut self, _cx: &mut Ctx, _b: Button) -> Tr {
        Tr::None
    }
    fn release(&mut self, _cx: &mut Ctx, _b: Button) -> Tr {
        Tr::None
    }
    fn update(&mut self, _cx: &mut Ctx, _dt: f64) -> Tr {
        Tr::None
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd);
    fn text(&mut self, _cx: &mut Ctx, _s: &str) -> Tr {
        Tr::None
    }
    fn key(&mut self, _cx: &mut Ctx, _k: Key) -> Tr {
        Tr::None
    }
    fn result(&mut self, _cx: &mut Ctx, _r: Res) -> Tr {
        Tr::None
    }
    /// Clicking a bezel icon (only the main screen responds).
    /// Whether clicking an icon may close this screen (and everything above the main
    /// screen) to open another. True for plain menus; false mid-game, mid-animation or
    /// mid-flow, where leaving would cut something short.
    fn leavable(&self) -> bool {
        false
    }
    fn activate_icon(&mut self, _cx: &mut Ctx, _i: usize) -> Tr {
        Tr::None
    }
    fn wants_text(&self) -> bool {
        false
    }
    fn is_main(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str;
}

pub fn boxed<S: Screen + 'static>(s: S) -> Box<dyn Screen> {
    Box::new(s)
}

/// A message followed by nothing else: shorthand for the common case.
pub fn msg(pages: &[(&str, &str)]) -> Box<dyn Screen> {
    boxed(Message::new(pages.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()))
}

pub fn msg_owned(pages: Vec<(String, String)>) -> Box<dyn Screen> {
    boxed(Message::new(pages))
}

// ====================================================================== Message
/// Pages of two 8-character lines. B advances, C skips to the end. Pops with `Res::Done`.
pub struct Message {
    pages: Vec<(String, String)>,
    i: usize,
    t: f64,
    sound: Option<Tune>,
}

impl Message {
    pub fn new(pages: Vec<(String, String)>) -> Message {
        Message { pages, i: 0, t: 0.0, sound: None }
    }
    pub fn with_sound(mut self, t: Tune) -> Message {
        self.sound = Some(t);
        self
    }
}

impl Screen for Message {
    fn name(&self) -> &'static str {
        "Message"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if let Some(s) = self.sound.take() {
            cx.play(s);
        }
        self.t += dt;
        Tr::None
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::B => {
                cx.play(Tune::Click);
                self.i += 1;
                self.t = 0.0;
                if self.i >= self.pages.len() {
                    return Tr::PopWith(Res::Done);
                }
                Tr::None
            }
            Button::C => Tr::PopWith(Res::Done),
            Button::A => Tr::None,
        }
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        if let Some((a, b)) = self.pages.get(self.i) {
            lcd.line(a, 1, self.t, true);
            lcd.line(b, 9, self.t, true);
            if self.i + 1 < self.pages.len() && (self.t * 2.0) as i32 % 2 == 1 {
                lcd.dot(31, 15);
            }
        }
    }
}

// ====================================================================== Confirm
/// A yes/no question. Pops with `Res::Yes(bool)`.
pub struct Confirm {
    q: String,
    t: f64,
}

impl Confirm {
    pub fn new(q: impl Into<String>) -> Confirm {
        Confirm { q: q.into(), t: 0.0 }
    }
}

impl Screen for Confirm {
    fn name(&self) -> &'static str {
        "Confirm"
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        Tr::None
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::B => {
                cx.play(Tune::Ok);
                Tr::PopWith(Res::Yes(true))
            }
            Button::C => {
                cx.play(Tune::Click);
                Tr::PopWith(Res::Yes(false))
            }
            Button::A => Tr::None,
        }
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        lcd.line(&self.q, 1, self.t, true);
        lcd.text("B:Y C:N", 2, 9);
    }
}

// ====================================================================== list helpers
/// A two-row list with a cursor (A = next). Used by menus.
#[derive(Default)]
pub struct ListView {
    pub i: usize,
    pub t: f64,
}

impl ListView {
    pub fn next(&mut self, n: usize) {
        if n > 0 {
            self.i = (self.i + 1) % n;
            self.t = 0.0;
        }
    }
    pub fn draw(&self, labels: &[String], lcd: &mut Lcd) {
        if labels.is_empty() {
            return;
        }
        let i = self.i % labels.len();
        let lab = &labels[i];
        if lab.chars().count() > 7 {
            lcd.line(lab, 1, self.t, false);
            lcd.clear_rect(0, 0, 4, 7);
        } else {
            lcd.text(lab, 4, 1);
        }
        lcd.text(">", 0, 1);
        if labels.len() > 1 {
            let next: String = labels[(i + 1) % labels.len()].chars().take(7).collect();
            lcd.text(&next, 4, 9);
        }
        lcd.dot(0, if i == labels.len() - 1 { 15 } else { 14 });
    }
}

/// One item per screen as two lines; a dot on the bottom row shows the position.
pub fn draw_pick(lcd: &mut Lcd, l1: &str, l2: &str, t: f64, i: usize, n: usize) {
    lcd.line(l1, 1, t, true);
    lcd.line(l2, 9, t, true);
    if n > 1 {
        let x = (i as i32 * 31) / (n as i32 - 1).max(1);
        lcd.dot(x, 15);
    }
}

/// A generic "pick one" screen over fixed rows. Pops with `Res::Pick(index)` or `Res::Cancel`.
pub struct Pick {
    rows: Vec<(String, String)>,
    empty: (String, String),
    i: usize,
    t: f64,
}

impl Pick {
    pub fn new(rows: Vec<(String, String)>) -> Pick {
        Pick { rows, empty: ("NOTHING".into(), String::new()), i: 0, t: 0.0 }
    }
    pub fn empty(mut self, a: &str, b: &str) -> Pick {
        self.empty = (a.into(), b.into());
        self
    }
}

impl Screen for Pick {
    fn name(&self) -> &'static str {
        "Pick"
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        Tr::None
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::A if !self.rows.is_empty() => {
                cx.play(Tune::Click);
                self.i = (self.i + 1) % self.rows.len();
                self.t = 0.0;
                Tr::None
            }
            Button::B if !self.rows.is_empty() => {
                cx.play(Tune::Click);
                Tr::PopWith(Res::Pick(self.i))
            }
            Button::C => {
                cx.play(Tune::Click);
                Tr::PopWith(Res::Cancel)
            }
            _ => Tr::None,
        }
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        match self.rows.get(self.i) {
            Some((a, b)) => draw_pick(lcd, a, b, self.t, self.i, self.rows.len()),
            None => draw_pick(lcd, &self.empty.0, &self.empty.1, self.t, 0, 0),
        }
    }
}

// ====================================================================== Name entry
pub struct NameEntry {
    name: String,
    k: usize,
    t: f64,
}

const NAME_CHOICES: &[&str] = &[
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W",
    "X", "Y", "Z", "<", "END",
];
const RANDOM_NAMES: &[&str] =
    &["MOCHI", "PIXEL", "BEAN", "NUBBIN", "TOFU", "ZIGGY", "PUDDLE", "BLIP", "KIWI", "SPROUT", "BOBA", "TATER"];

impl NameEntry {
    pub fn new() -> NameEntry {
        NameEntry { name: String::new(), k: 0, t: 0.0 }
    }
    fn done(&mut self, cx: &mut Ctx) -> Tr {
        if self.name.is_empty() {
            self.name = cx.game.rng.pick(RANDOM_NAMES).unwrap_or(&"MOCHI").to_string();
        }
        cx.play(Tune::Ok);
        Tr::PopWith(Res::Name(self.name.clone()))
    }
}

impl Default for NameEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for NameEntry {
    fn name(&self) -> &'static str {
        "NameEntry"
    }
    fn wants_text(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        Tr::None
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        cx.play(Tune::Click);
        match b {
            Button::A => self.k = (self.k + 1) % NAME_CHOICES.len(),
            Button::C => {
                self.name.pop();
            }
            Button::B => match NAME_CHOICES[self.k] {
                "END" => return self.done(cx),
                "<" => {
                    self.name.pop();
                }
                ch => {
                    if self.name.len() < 7 {
                        self.name.push_str(ch);
                    }
                }
            },
        }
        Tr::None
    }
    fn text(&mut self, _cx: &mut Ctx, s: &str) -> Tr {
        for ch in s.chars() {
            if ch.is_ascii_alphabetic() && self.name.len() < 7 {
                self.name.push(ch.to_ascii_uppercase());
            }
        }
        Tr::None
    }
    fn key(&mut self, cx: &mut Ctx, k: Key) -> Tr {
        match k {
            Key::Backspace => {
                self.name.pop();
                Tr::None
            }
            Key::Enter => self.done(cx),
        }
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        if self.name.is_empty() {
            lcd.text("NAME?", 0, 1);
        } else {
            lcd.text(&self.name, 0, 1);
            if (self.t * 3.0) as i32 % 2 == 1 {
                let x = self.name.len() as i32 * 4;
                lcd.hline(x, x + 2, 6);
            }
        }
        lcd.text(&format!(">{}", NAME_CHOICES[self.k]), 0, 9);
    }
}

/// Four heart slots.
pub fn draw_hearts(lcd: &mut Lcd, c: &Content, n: u8, y: i32) {
    for i in 0..4 {
        let s = if i < n { c.prop("heart") } else { c.prop("heart_empty") };
        lcd.draw(s, i as i32 * 8, y);
    }
}
