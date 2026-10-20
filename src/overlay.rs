//! The right-click settings menu, drawn inside the widget (so it works the same on every OS,
//! with no native menu toolkit). Items that only the desktop shell can carry out are returned
//! as `DesktopCmd`s.

use crate::core::Core;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Closed,
    Menu,
    Help,
}

#[derive(Default, Debug, Clone)]
pub struct Overlay {
    pub page: Page,
    pub hover: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Size,
    Shell,
    Speed,
    Sound,
    OnTop,
    Notify,
    Login,
    Help,
    Hide,
    Quit,
    Close,
}

/// What the desktop shell must do after a menu choice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DesktopCmd {
    SetScale(f32),
    SetOnTop(bool),
    SetLogin(bool),
    Hide,
    Quit,
}

/// Facts about the desktop the menu needs to show.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    pub login: bool,
    /// "HIDE TO TRAY" when a tray icon exists, otherwise "MINIMIZE".
    pub hide_label: &'static str,
}

pub struct Item {
    pub label: &'static str,
    pub value: String,
    pub act: Act,
}

pub const SIZES: [(&str, f32); 4] = [("SMALL", 0.40), ("MEDIUM", 0.55), ("LARGE", 0.75), ("HUGE", 1.0)];

pub fn size_name(scale: f32) -> &'static str {
    SIZES.iter().min_by(|a, b| (a.1 - scale).abs().total_cmp(&(b.1 - scale).abs())).map(|s| s.0).unwrap_or("MEDIUM")
}

pub fn items(core: &Core, env: &Env) -> Vec<Item> {
    let s = &core.game.settings;
    let onoff = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
    vec![
        Item { label: "SIZE", value: size_name(s.ui.scale).into(), act: Act::Size },
        Item { label: "SHELL", value: crate::layout::shell(s.shell).name.into(), act: Act::Shell },
        Item { label: "SPEED", value: format!("{}X", s.speed), act: Act::Speed },
        Item { label: "SOUND", value: onoff(!s.muted), act: Act::Sound },
        Item { label: "ON TOP", value: onoff(s.ui.on_top), act: Act::OnTop },
        Item { label: "NOTIFY", value: onoff(s.ui.notify), act: Act::Notify },
        Item { label: "AT LOGIN", value: onoff(env.login), act: Act::Login },
        Item { label: "HELP", value: String::new(), act: Act::Help },
        Item { label: env.hide_label, value: String::new(), act: Act::Hide },
        Item { label: "QUIT", value: String::new(), act: Act::Quit },
        Item { label: "CLOSE", value: String::new(), act: Act::Close },
    ]
}

pub const HELP: &[&str] = &[
    "A  Z LEFT 1",
    "B  X ENTER 2",
    "C  C RIGHT 3",
    "A+C OR M  MUTE",
    "",
    "DRAG TO MOVE",
    "RIGHT-CLICK MENU",
    "F2 SPEED F3 SHELL",
    "",
    "B ON MAIN: CLOCK",
    "C ON MAIN: CALL PET",
];

impl Overlay {
    pub fn open(&mut self) {
        self.page = Page::Menu;
        self.hover = Some(0);
    }
    pub fn is_open(&self) -> bool {
        self.page != Page::Closed
    }
    pub fn close(&mut self) {
        self.page = Page::Closed;
        self.hover = None;
    }
}

/// Carry out a menu choice. Settings stored in the game are changed here; the rest is
/// returned for the desktop shell.
pub fn activate(core: &mut Core, env: &Env, act: Act) -> Option<DesktopCmd> {
    let s = &mut core.game.settings;
    match act {
        Act::Size => {
            let i = SIZES.iter().position(|x| x.0 == size_name(s.ui.scale)).unwrap_or(1);
            let next = SIZES[(i + 1) % SIZES.len()].1;
            s.ui.scale = next;
            return Some(DesktopCmd::SetScale(next));
        }
        Act::Shell => core.next_shell(),
        Act::Speed => core.cycle_speed(),
        Act::Sound => core.toggle_mute(),
        Act::OnTop => {
            s.ui.on_top = !s.ui.on_top;
            return Some(DesktopCmd::SetOnTop(s.ui.on_top));
        }
        Act::Notify => s.ui.notify = !s.ui.notify,
        Act::Login => return Some(DesktopCmd::SetLogin(!env.login)),
        Act::Help => core.overlay.page = Page::Help,
        Act::Hide => {
            core.overlay.close();
            return Some(DesktopCmd::Hide);
        }
        Act::Quit => return Some(DesktopCmd::Quit),
        Act::Close => core.overlay.close(),
    }
    None
}

/// Panel layout in window pixels: (x, y, w, h, row height, font scale).
pub fn panel(win_w: f32, win_h: f32, rows: usize, dpi: f32) -> (f32, f32, f32, f32, f32, f32) {
    let fs = (2.0 * dpi).round().max(1.0);
    let row_h = 8.0 * fs;
    let w = (21.0 * 4.0 * fs + 6.0 * fs).min(win_w - 4.0);
    let h = rows as f32 * row_h + 3.0 * fs + row_h;
    let x = ((win_w - w) / 2.0).max(2.0);
    let y = ((win_h - h) / 2.0).max(2.0);
    (x, y, w, h, row_h, fs)
}

/// Which menu row is under a window-pixel position.
pub fn row_at(px: f32, py: f32, win_w: f32, win_h: f32, rows: usize, dpi: f32) -> Option<usize> {
    let (x, y, w, _h, row_h, fs) = panel(win_w, win_h, rows, dpi);
    let top = y + row_h + 1.5 * fs;
    if px < x || px > x + w || py < top {
        return None;
    }
    let r = ((py - top) / row_h) as usize;
    (r < rows).then_some(r)
}
