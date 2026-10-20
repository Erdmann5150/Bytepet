//! The right-click settings menu, drawn inside the widget (so it works the same on every OS,
//! with no native menu toolkit). Items that only the desktop shell can carry out are returned
//! as `DesktopCmd`s.
//!
//! The SHELL row opens a page of its own: a preset, then one page per part of the device
//! (body, buttons, antenna, screen) where each setting steps through its options.

use crate::core::Core;
use crate::look::{Field, Look, Part};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Closed,
    Menu,
    Help,
    /// The preset and the list of parts.
    Shell,
    Part(Part),
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
    /// The next preset look.
    Preset,
    /// Open a part's page.
    Part(Part),
    /// The next option of one setting.
    Field(Field),
    Random,
    /// Up one page.
    Back,
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
    let back = Item { label: "BACK", value: String::new(), act: Act::Back };
    match core.overlay.page {
        Page::Shell => {
            let mut v = vec![Item { label: "PRESET", value: s.shell_name().into(), act: Act::Preset }];
            v.extend(Part::ALL.iter().map(|&p| Item { label: p.name(), value: ">".into(), act: Act::Part(p) }));
            v.push(Item { label: "RANDOM", value: String::new(), act: Act::Random });
            v.push(back);
            return v;
        }
        Page::Part(p) => {
            let mut v: Vec<Item> = p
                .fields()
                .iter()
                .map(|&f| Item { label: f.label(), value: s.look.value_name(f).into(), act: Act::Field(f) })
                .collect();
            v.push(back);
            return v;
        }
        Page::Menu | Page::Help | Page::Closed => {}
    }
    let onoff = |b: bool| if b { "ON".to_string() } else { "OFF".to_string() };
    vec![
        Item { label: "SIZE", value: size_name(s.ui.scale).into(), act: Act::Size },
        Item { label: "SHELL", value: s.shell_name().into(), act: Act::Shell },
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
pub const COLS: usize = 15;
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
    fn go(&mut self, page: Page, hover: usize) {
        self.page = page;
        self.hover = Some(hover);
    }
    /// Up one page: a part's page goes back to the SHELL page, that to the menu, and the menu
    /// (or help) closes.
    pub fn back(&mut self) {
        match self.page {
            Page::Part(p) => self.go(Page::Shell, 1 + p as usize),
            Page::Shell => self.go(Page::Menu, 1),
            _ => self.close(),
        }
    }
    /// Is this a page of rows to pick from?
    pub fn has_rows(&self) -> bool {
        matches!(self.page, Page::Menu | Page::Shell | Page::Part(_))
    }
    /// How many rows the page has.
    pub fn rows(&self) -> usize {
        match self.page {
            Page::Menu => ROWS,
            Page::Shell => Part::ALL.len() + 3,
            Page::Part(p) => p.fields().len() + 1,
            Page::Help | Page::Closed => 0,
        }
    }
    /// The panel's title.
    pub fn title(&self) -> &'static str {
        match self.page {
            Page::Closed | Page::Menu => "BYTEPET",
            Page::Help => HELP_PAGES[self.help.min(HELP_PAGES.len() - 1)].0,
            Page::Shell => "SHELL",
            Page::Part(p) => p.name(),
        }
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
        Act::Shell => core.overlay.go(Page::Shell, 0),
        Act::Preset => s.next_shell(),
        Act::Part(p) => core.overlay.go(Page::Part(p), 0),
        Act::Field(f) => s.look.step(f, 1),
        Act::Random => {
            let mut rng = crate::rng::Pcg32::new(crate::core::now_unix().to_bits());
            let old = s.look;
            while s.look == old {
                s.look = Look::random(&mut rng);
            }
        }
        Act::Back => core.overlay.back(),
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

/// Left and right on the keyboard: step the highlighted setting back or forward. Rows that
/// aren't a setting with options ignore it.
pub fn adjust(core: &mut Core, env: &Env, dir: i32) {
    let Some(hover) = core.overlay.hover else { return };
    let Some(act) = items(core, env).get(hover).map(|it| it.act) else { return };
    let s = &mut core.game.settings;
    match act {
        Act::Field(f) => s.look.step(f, dir),
        Act::Preset => {
            let n = crate::look::PRESETS.len() as i32;
            s.set_shell((s.shell as i32 + dir).rem_euclid(n) as u32);
        }
        _ => {}
    }
}

/// A setting picked by value, as the tray menu offers them (the menu above steps through values).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// An index into [`SIZES`].
    Size(usize),
    /// A preset look.
    Shell(u32),
    /// One option of one part of the look.
    Look(Field, usize),
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
        Choice::Shell(i) => s.set_shell(i),
        Choice::Look(f, i) => s.look.set(f, i),
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

impl Overlay {
    /// Where this page's panel sits. The menu and help fill most of the window; the SHELL
    /// pages are only as tall as their rows and keep to the top or bottom, away from the part
    /// being changed, so you can watch it change.
    pub fn panel(&self, win_w: f32, win_h: f32) -> Panel {
        let full = panel(win_w, win_h);
        let at_bottom = match self.page {
            // the icons' panel covers the top row and leaves the bottom row, with the alert icon
            // and the name, in sight
            Page::Shell | Page::Part(Part::Body | Part::Buttons | Part::Icons) => false,
            Page::Part(Part::Antenna | Part::Screen) => true,
            Page::Closed | Page::Menu | Page::Help => return full,
        };
        let h = ((self.rows() + 1) as f32 * ROW_DOTS + 3.0) * full.fs;
        let y = if at_bottom { win_h - h - full.y } else { full.y }.round().max(0.0);
        Panel { y, h, top: y + 1.5 * full.fs + full.row_h, ..full }
    }
}

/// Which row of the open page is under a window-pixel position.
pub fn row_at(o: &Overlay, px: f32, py: f32, win_w: f32, win_h: f32) -> Option<usize> {
    let p = o.panel(win_w, win_h);
    if !p.contains(px, py) || py < p.top {
        return None;
    }
    let r = ((py - p.top) / p.row_h) as usize;
    (r < o.rows()).then_some(r)
}

/// A left click at a window-pixel position while the menu is open. A click on a row chooses
/// it, a click on a help page turns the page, and a click outside the panel closes the menu.
pub fn click(core: &mut Core, env: &Env, px: f32, py: f32, win_w: f32, win_h: f32) -> Option<DesktopCmd> {
    match core.overlay.page {
        Page::Closed => None,
        Page::Help => {
            if core.overlay.panel(win_w, win_h).contains(px, py) {
                core.overlay.next_help();
            } else {
                core.overlay.close();
            }
            None
        }
        Page::Menu | Page::Shell | Page::Part(_) => {
            if let Some(r) = row_at(&core.overlay, px, py, win_w, win_h) {
                core.overlay.hover = Some(r);
                let act = items(core, env)[r].act;
                activate(core, env, act)
            } else {
                if !core.overlay.panel(win_w, win_h).contains(px, py) {
                    core.overlay.close();
                }
                None
            }
        }
    }
}

/// Mouse wheel over the menu: positive `steps` move the highlight down.
pub fn scroll(core: &mut Core, steps: i32) {
    if core.overlay.has_rows() {
        let n = core.overlay.rows() as i32;
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
            let fits = |it: &Item| {
                let gap = if it.value.is_empty() { 0 } else { 8 };
                let w = text_width(it.label) + gap + text_width(&it.value);
                assert!(w <= content, "{} {} is too wide", it.label, it.value);
            };
            for &(_, s) in &SIZES {
                core.game.settings.ui.scale = s;
                let items = items(&core, &env);
                assert_eq!(items.len(), ROWS);
                items.iter().for_each(fits);
            }
            // every preset, and CUSTOM, on the menu and the SHELL page
            for shell in 0..=crate::look::PRESETS.len() as u32 {
                core.game.settings.set_shell(shell);
                if shell as usize == crate::look::PRESETS.len() {
                    core.game.settings.look.step(Field::Ring, 1);
                }
                for page in [Page::Menu, Page::Shell] {
                    core.overlay.page = page;
                    items(&core, &env).iter().for_each(fits);
                }
            }
            // every option of every part
            for part in Part::ALL {
                core.overlay.page = Page::Part(part);
                assert_eq!(items(&core, &env).len(), core.overlay.rows());
                for &f in part.fields() {
                    for i in 0..f.count() {
                        core.game.settings.look.set(f, i);
                        items(&core, &env).iter().for_each(fits);
                    }
                }
            }
            core.overlay.page = Page::Shell;
            assert_eq!(items(&core, &env).len(), core.overlay.rows());
            assert!(core.overlay.rows() <= ROWS);
            core.overlay.page = Page::Closed;
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
    fn the_shell_pages() {
        let content = crate::content::Content::builtin();
        let game = crate::sim::Game::new(&content, crate::rng::Pcg32::new(1));
        let mut core = Core::new(content, game);
        let env = Env { login: false, hide_label: "MINIMIZE" };
        let (w, h) = crate::render::window_size(0.55);
        let (w, h) = (w as f32, h as f32);
        // rows of whichever page is open
        let row = |core: &Core, i: usize| {
            let p = core.overlay.panel(w, h);
            (p.x + p.w / 2.0, p.top + (i as f32 + 0.5) * p.row_h)
        };
        let click_row = |core: &mut Core, i: usize| {
            let (x, y) = row(core, i);
            click(core, &env, x, y, w, h)
        };
        core.overlay.open();
        // SHELL opens its page; PRESET steps through the presets
        click_row(&mut core, 1);
        assert_eq!(core.overlay.page, Page::Shell);
        click_row(&mut core, 0);
        assert_eq!(core.game.settings.shell_name(), "LIME");
        // BUTTONS > SHAPE changes just the buttons
        click_row(&mut core, 2);
        assert_eq!(core.overlay.page, Page::Part(Part::Buttons));
        assert_eq!(items(&core, &env)[1].value, "ROUND");
        click_row(&mut core, 1);
        assert_eq!(items(&core, &env)[1].value, "SQUARE");
        assert_eq!(core.game.settings.look.body, crate::look::PRESETS[1].look.body);
        assert_eq!(core.game.settings.shell_name(), "CUSTOM");
        // left and right step either way
        core.overlay.hover = Some(1);
        adjust(&mut core, &env, -1);
        assert_eq!(core.game.settings.shell_name(), "LIME");
        // a short page's panel is only as tall as its rows, and stays clear of what it changes
        let p = core.overlay.panel(w, h);
        assert!(p.h < panel(w, h).h / 2.0, "{p:?}");
        let (x, y) = row(&core, 4);
        assert_eq!(row_at(&core.overlay, x, y, w, h), None);
        use crate::layout::{BUTTONS, STAR, VIEW};
        let scale = w / VIEW.2;
        let px = |x: f32, y: f32| ((x - VIEW.0) * scale, (y - VIEW.1) * scale);
        for &(_, bx, by) in &BUTTONS {
            let (x, y) = px(bx, by);
            assert!(!p.contains(x, y), "the panel hides the buttons");
        }
        core.overlay.go(Page::Part(Part::Antenna), 0);
        let (x, y) = px(STAR.0, STAR.1);
        assert!(!core.overlay.panel(w, h).contains(x, y), "the panel hides the antenna");
        core.overlay.go(Page::Part(Part::Icons), 0);
        let p = core.overlay.panel(w, h);
        for &(x, y, iw, ih) in &crate::layout::icon_rects()[5..] {
            let (x, y) = px(x + iw / 2.0, y + ih / 2.0);
            assert!(!p.contains(x, y), "the panel hides the bottom icons");
        }
        core.overlay.go(Page::Part(Part::Buttons), 1);
        // BACK, then right-click, then right-click again: up a page each time
        let n = core.overlay.rows();
        click_row(&mut core, n - 1);
        assert_eq!((core.overlay.page, core.overlay.hover), (Page::Shell, Some(2)));
        core.right_click();
        assert_eq!((core.overlay.page, core.overlay.hover), (Page::Menu, Some(1)));
        core.right_click();
        assert!(!core.overlay.is_open());
        // RANDOM always changes something
        core.overlay.open();
        core.overlay.go(Page::Shell, 0);
        for _ in 0..20 {
            let before = core.game.settings.look;
            activate(&mut core, &env, Act::Random);
            assert_ne!(core.game.settings.look, before);
        }
        // the scroll wheel wraps within the page
        core.overlay.go(Page::Part(Part::Antenna), 0);
        scroll(&mut core, -1);
        assert_eq!(core.overlay.hover, Some(3));
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
        choose(&mut core, Choice::Look(Field::Lcd, 3));
        choose(&mut core, Choice::Speed(60));
        choose(&mut core, Choice::Sound(None));
        assert_eq!(choose(&mut core, Choice::OnTop(false)), Some(DesktopCmd::SetOnTop(false)));
        choose(&mut core, Choice::Notify(false));
        let s = &core.game.settings;
        assert_eq!((s.shell, s.speed, s.muted, s.ui.on_top, s.ui.notify), (5, 60, true, false, false));
        assert_eq!(s.look.lcd, crate::look::Lcd::Amber);
        assert_eq!(s.look.pattern, crate::look::Pattern::Flame);
        assert_eq!(s.shell_name(), "CUSTOM");
        assert_eq!(choose(&mut core, Choice::Login(true)), Some(DesktopCmd::SetLogin(true)));
    }
}
