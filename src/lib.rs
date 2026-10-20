//! Bytepet core: everything except the operating-system shell.
//!
//! This library has no system dependencies, so it builds and tests anywhere. The desktop
//! binary (`src/main.rs`, feature `desktop`) adds the window, tray, sound and notifications.

pub mod content;
pub mod core;
pub mod font;
pub mod games;
pub mod instance;
pub mod layout;
pub mod lcd;
pub mod link;
pub mod net;
pub mod overlay;
pub mod render;
pub mod rng;
pub mod save;
pub mod screens;
pub mod sim;
pub mod sprite;

/// The three device buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    A,
    B,
    C,
}

impl Button {
    pub fn index(self) -> usize {
        match self {
            Button::A => 0,
            Button::B => 1,
            Button::C => 2,
        }
    }
    pub const ALL: [Button; 3] = [Button::A, Button::B, Button::C];
}

/// Short beep melodies, played by the desktop shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tune {
    Click,
    Ok,
    No,
    Call,
    Hatch,
    Evolve,
    Win,
    Lose,
    Mail,
    Tick,
    Hit,
    Gone,
    Wed,
}

impl Tune {
    /// (frequency Hz, milliseconds); frequency 0 is a rest.
    pub fn notes(self) -> &'static [(u32, u32)] {
        match self {
            Tune::Click => &[(2400, 22)],
            Tune::Ok => &[(1800, 40), (2400, 60)],
            Tune::No => &[(380, 140)],
            Tune::Call => &[(2200, 80), (0, 60), (2200, 80), (0, 60), (2200, 80)],
            Tune::Hatch => &[(1047, 90), (1319, 90), (1568, 90), (2093, 200)],
            Tune::Evolve => &[(1568, 70), (1319, 70), (1568, 70), (2093, 70), (2637, 220)],
            Tune::Win => &[(1568, 90), (2093, 90), (2637, 180)],
            Tune::Lose => &[(784, 160), (523, 260)],
            Tune::Mail => &[(2637, 60), (0, 30), (3136, 120)],
            Tune::Tick => &[(3000, 10)],
            Tune::Hit => &[(300, 90)],
            Tune::Gone => &[(1568, 200), (1319, 200), (1047, 200), (784, 400)],
            Tune::Wed => &[(1319, 120), (1568, 120), (2093, 120), (1568, 120), (2093, 120), (2637, 300)],
        }
    }
}
