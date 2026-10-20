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
    /// Which of [`HELP_PAGES`] is showing.
    pub help: usize,
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
        Item { label: "SOUND", value: s.sound_label(), act: Act::Sound },
        Item { label: "ON TOP", value: onoff(s.ui.on_top), act: Act::OnTop },
        Item { label: "NOTIFY", value: onoff(s.ui.notify), act: Act::Notify },
        Item { label: "AT LOGIN", value: onoff(env.login), act: Act::Login },
        Item { label: "HELP", value: String::new(), act: Act::Help },
        Item { label: env.hide_label, value: String::new(), act: Act::Hide },
        Item { label: "QUIT", value: String::new(), act: Act::Quit },
        Item { label: "CLOSE", value: String::new(), act: Act::Close },
    ]
}

/// The help pages: a title and up to [`ROWS`] lines of at most [`COLS`] characters.
pub const HELP_PAGES: [(&str, &[&str]); 2] = [
    (
        "KEYS 1/2",
        &["A  Z  LEFT", "B  X  ENTER", "C  C  RIGHT", "", "M/A+C  MUTE", "F1  HELP", "F2  SPEED", "F3  SHELL"],
    ),
    (
        "MOUSE 2/2",
        &[
            "CLICK ABC",
            "OR AN ICON",
            "",
            "SCREEN   B",
            "WHEEL  A/BACK",
            "R-CLICK  C",
            "",
            "AT HOME,",
            "R-CLICK  MENU",
            "",
            "DRAG TO MOVE",
        ],
    ),
];

/// Characters per line: the longest label, two spaces and its value must fit.
pub const COLS: usize = 13;
/// Lines per page: the menu's items. Help pages use the same panel.
pub const ROWS: usize = 11;

impl Overlay {
    pub fn open(&mut self) {
        self.page = Page::Menu;
        self.hover = Some(0);
        self.help = 0;
    }
    pub fn open_help(&mut self) {
        self.page = Page::Help;
        self.help = 0;
    }
    /// Show the next help page, or go back to the menu after the last one.
    pub fn next_help(&mut self) {
        if self.help + 1 < HELP_PAGES.len() {
            self.help += 1;
        } else {
            self.page = Page::Menu;
            self.help = 0;
            self.hover.get_or_insert(0);
        }
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
        Act::Sound => core.cycle_sound(),
        Act::OnTop => {
            s.ui.on_top = !s.ui.on_top;
            return Some(DesktopCmd::SetOnTop(s.ui.on_top));
        }
        Act::Notify => s.ui.notify = !s.ui.notify,
        Act::Login => return Some(DesktopCmd::SetLogin(!env.login)),
        Act::Help => core.overlay.open_help(),
        Act::Hide => {
            core.overlay.close();
            return Some(DesktopCmd::Hide);
        }
        Act::Quit => return Some(DesktopCmd::Quit),
        Act::Close => core.overlay.close(),
    }
    None
}

/// A setting picked by value, as the tray menu offers them (the menu above steps through values).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// An index into [`SIZES`].
    Size(usize),
    Shell(u32),
    Speed(u32),
    /// `None` is off, otherwise a volume in percent.
    Sound(Option<u32>),
    OnTop(bool),
    Notify(bool),
    Login(bool),
}

/// Apply a [`Choice`]. Like [`activate`], what only the desktop shell can do is returned.
pub fn choose(core: &mut Core, c: Choice) -> Option<DesktopCmd> {
    let s = &mut core.game.settings;
    match c {
        Choice::Size(i) => {
            let scale = SIZES[i.min(SIZES.len() - 1)].1;
            s.ui.scale = scale;
            return Some(DesktopCmd::SetScale(scale));
        }
        Choice::Shell(i) => s.shell = i % crate::layout::SHELLS.len() as u32,
        Choice::Speed(x) => s.speed = x,
        Choice::Sound(v) => core.set_sound(v),
        Choice::OnTop(on) => {
            s.ui.on_top = on;
            return Some(DesktopCmd::SetOnTop(on));
        }
        Choice::Notify(on) => s.ui.notify = on,
        Choice::Login(on) => return Some(DesktopCmd::SetLogin(on)),
    }
    None
}

/// Where the menu panel sits, in window pixels. Text is drawn in whole-pixel dots of size
/// `fs`, the largest that lets the panel fill most of the window, so the menu grows and
/// shrinks with the device and stays crisp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Pixels per font dot.
    pub fs: f32,
    pub row_h: f32,
    /// Top of the first row (below the title).
    pub top: f32,
}

/// Font dots: 1.5 padding, a 7-dot title row, ROWS rows of 7 dots, 1.5 padding.
const ROW_DOTS: f32 = 7.0;
const PANEL_W_DOTS: f32 = (COLS * 4 - 1) as f32 + 6.0;
const PANEL_H_DOTS: f32 = (ROWS + 1) as f32 * ROW_DOTS + 3.0;
/// Share of the window the panel may cover.
const FILL: f32 = 0.92;

pub fn panel(win_w: f32, win_h: f32) -> Panel {
    let fs = (win_w * FILL / PANEL_W_DOTS).min(win_h * FILL / PANEL_H_DOTS).floor().max(1.0);
    let (w, h) = (PANEL_W_DOTS * fs, PANEL_H_DOTS * fs);
    let x = ((win_w - w) / 2.0).round().max(0.0);
    let y = ((win_h - h) / 2.0).round().max(0.0);
    let row_h = ROW_DOTS * fs;
    Panel { x, y, w, h, fs, row_h, top: y + 1.5 * fs + row_h }
}

impl Panel {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
    /// The panel as whole pixels (x, y, w, h), for the window's click-through shape.
    pub fn pixels(&self) -> (i32, i32, i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32, self.w.ceil() as i32 + 1, self.h.ceil() as i32 + 1)
    }
}

/// Which menu row is under a window-pixel position.
pub fn row_at(px: f32, py: f32, win_w: f32, win_h: f32) -> Option<usize> {
    let p = panel(win_w, win_h);
    if !p.contains(px, py) || py < p.top {
        return None;
    }
    let r = ((py - p.top) / p.row_h) as usize;
    (r < ROWS).then_some(r)
}

/// A left click at a window-pixel position while the menu is open. A click on a row chooses
/// it, a click on a help page turns the page, and a click outside the panel closes the menu.
pub fn click(core: &mut Core, env: &Env, px: f32, py: f32, win_w: f32, win_h: f32) -> Option<DesktopCmd> {
    match core.overlay.page {
        Page::Closed => None,
        Page::Help => {
            if panel(win_w, win_h).contains(px, py) {
                core.overlay.next_help();
            } else {
                core.overlay.close();
            }
            None
        }
        Page::Menu => {
            if let Some(r) = row_at(px, py, win_w, win_h) {
                core.overlay.hover = Some(r);
                let act = items(core, env)[r].act;
                activate(core, env, act)
            } else {
                if !panel(win_w, win_h).contains(px, py) {
                    core.overlay.close();
                }
                None
            }
        }
    }
}

/// Mouse wheel over the menu: positive `steps` move the highlight down.
pub fn scroll(core: &mut Core, steps: i32) {
    if core.overlay.page == Page::Menu {
        let n = ROWS as i32;
        let h = core.overlay.hover.unwrap_or(0) as i32;
        core.overlay.hover = Some((h + steps).rem_euclid(n) as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::text_width;

    fn width_dots(chars: usize) -> i32 {
        chars as i32 * 4 - 1
    }

    #[test]
    fn the_panel_fits_and_stays_readable_at_every_size() {
        for &(name, scale) in &SIZES {
            for dpi in [1.0f32, 1.25, 1.5, 2.0] {
                let (w, h) = crate::render::window_size(scale * dpi);
                let p = panel(w as f32, h as f32);
                assert!(p.x >= 0.0 && p.y >= 0.0, "{name} @{dpi}: {p:?}");
                assert!(p.x + p.w <= w as f32 && p.y + p.h <= h as f32, "{name} @{dpi}: {p:?} in {w}x{h}");
                // 2 px per dot makes 10 px capitals: the smallest that reads comfortably
                assert!(p.fs >= 2.0 * dpi.floor(), "{name} @{dpi}: dots of {} px", p.fs);
            }
        }
        // bigger devices get bigger text
        let fs: Vec<f32> = SIZES
            .iter()
            .map(|&(_, s)| {
                let (w, h) = crate::render::window_size(s);
                panel(w as f32, h as f32).fs
            })
            .collect();
        assert!(fs.windows(2).all(|p| p[0] <= p[1]) && fs[0] < fs[3], "{fs:?}");
    }

    #[test]
    fn every_line_fits_the_panel() {
        let content = width_dots(COLS);
        for hide in ["HIDE TO TRAY", "MINIMIZE"] {
            let env = Env { login: false, hide_label: hide };
            let mut core = Core::new(
                crate::content::Content::builtin(),
                crate::sim::Game::new(&crate::content::Content::builtin(), crate::rng::Pcg32::new(1)),
            );
            for shell in 0..crate::screens::town::SHELL_COUNT {
                core.game.settings.shell = shell;
                for &(_, s) in &SIZES {
                    core.game.settings.ui.scale = s;
                    let items = items(&core, &env);
                    assert_eq!(items.len(), ROWS);
                    for it in &items {
                        let gap = if it.value.is_empty() { 0 } else { 8 };
                        let w = text_width(it.label) + gap + text_width(&it.value);
                        assert!(w <= content, "{} {} is too wide", it.label, it.value);
                    }
                }
            }
        }
        for (title, lines) in HELP_PAGES {
            assert!(lines.len() <= ROWS, "{title} has too many lines");
            for l in std::iter::once(&title).chain(lines.iter()) {
                assert!(text_width(l) <= content, "help line {l:?} is too wide");
            }
        }
    }

    #[test]
    fn clicking_rows_and_pages() {
        let content = crate::content::Content::builtin();
        let game = crate::sim::Game::new(&content, crate::rng::Pcg32::new(1));
        let mut core = Core::new(content, game);
        let env = Env { login: false, hide_label: "MINIMIZE" };
        let (w, h) = crate::render::window_size(0.4);
        let (w, h) = (w as f32, h as f32);
        let p = panel(w, h);
        let row = |i: usize| (p.x + p.w / 2.0, p.top + (i as f32 + 0.5) * p.row_h);
        core.overlay.open();
        // SOUND row toggles the sound
        let muted = core.game.settings.muted;
        let (x, y) = row(3);
        assert_eq!(click(&mut core, &env, x, y, w, h), None);
        assert_ne!(core.game.settings.muted, muted);
        // HELP opens page 1, clicks turn the pages, then back to the menu
        let (x, y) = row(7);
        click(&mut core, &env, x, y, w, h);
        assert_eq!((core.overlay.page, core.overlay.help), (Page::Help, 0));
        click(&mut core, &env, x, y, w, h);
        assert_eq!((core.overlay.page, core.overlay.help), (Page::Help, 1));
        click(&mut core, &env, x, y, w, h);
        assert_eq!(core.overlay.page, Page::Menu);
        // the SIZE row asks the shell to resize
        let (x, y) = row(0);
        assert!(matches!(click(&mut core, &env, x, y, w, h), Some(DesktopCmd::SetScale(_))));
        // a click outside the panel closes the menu
        click(&mut core, &env, 1.0, 1.0, w, h);
        assert!(!core.overlay.is_open());
    }

    #[test]
    fn sound_steps_through_the_volumes() {
        let content = crate::content::Content::builtin();
        let game = crate::sim::Game::new(&content, crate::rng::Pcg32::new(1));
        let mut core = Core::new(content, game);
        let env = Env { login: false, hide_label: "MINIMIZE" };
        let sound = |core: &Core| items(core, &env)[3].value.clone();
        assert_eq!(sound(&core), "100%");
        let mut seen = Vec::new();
        for _ in 0..5 {
            activate(&mut core, &env, Act::Sound);
            seen.push(sound(&core));
        }
        assert_eq!(seen, ["OFF", "25%", "50%", "75%", "100%"]);
        // muting keeps the volume for when the sound comes back
        choose(&mut core, Choice::Sound(Some(50)));
        core.toggle_mute();
        assert_eq!(sound(&core), "OFF");
        core.toggle_mute();
        assert_eq!(sound(&core), "50%");
    }

    #[test]
    fn choices_set_values_directly() {
        let content = crate::content::Content::builtin();
        let game = crate::sim::Game::new(&content, crate::rng::Pcg32::new(1));
        let mut core = Core::new(content, game);
        assert_eq!(choose(&mut core, Choice::Size(3)), Some(DesktopCmd::SetScale(1.0)));
        assert_eq!(core.game.settings.ui.scale, 1.0);
        choose(&mut core, Choice::Shell(5));
        choose(&mut core, Choice::Speed(60));
        choose(&mut core, Choice::Sound(None));
        assert_eq!(choose(&mut core, Choice::OnTop(false)), Some(DesktopCmd::SetOnTop(false)));
        choose(&mut core, Choice::Notify(false));
        let s = &core.game.settings;
        assert_eq!((s.shell, s.speed, s.muted, s.ui.on_top, s.ui.notify), (5, 60, true, false, false));
        assert_eq!(choose(&mut core, Choice::Login(true)), Some(DesktopCmd::SetLogin(true)));
    }
}
