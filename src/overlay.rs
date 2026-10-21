//! The settings, shown on the device's own screen. While they're open the LCD switches to twice
//! its resolution (a [`GRID_W`]x[`GRID_H`] dot grid) and shows a page of rows, in the screen's
//! own colours, so the rest of the device stays in sight while you change it. The device's
//! buttons work it: A moves to the next row, B changes or opens it, C goes back. Items that only
//! the desktop shell can carry out are returned as `DesktopCmd`s.
//!
//! The SHELL row opens a page of its own: a preset, then one page per part of the device
//! (body, buttons, antenna, screen, icons) where each setting steps through its options.

use crate::core::Core;
use crate::font::{for_each_dot, text_width};
use crate::layout::{CELL, LCD_POS, VIEW};
use crate::look::{Field, Look, Part};
use crate::screens::town::SPEEDS;
use crate::{Button, Tune};

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
    /// Facts about the desktop; the desktop shell keeps them up to date.
    pub env: Env,
    /// Commands from button presses (see `Core::press`) for the desktop shell to carry out.
    cmds: Vec<DesktopCmd>,
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

impl Default for Env {
    fn default() -> Self {
        Env { login: false, hide_label: "MINIMIZE" }
    }
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

pub fn items(core: &Core) -> Vec<Item> {
    let s = &core.game.settings;
    let env = &core.overlay.env;
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
        Item { label: "HELP", value: ">".into(), act: Act::Help },
        Item { label: env.hide_label, value: String::new(), act: Act::Hide },
        Item { label: "QUIT", value: String::new(), act: Act::Quit },
        Item { label: "CLOSE", value: String::new(), act: Act::Close },
    ]
}

/// The help pages: a title and up to [`VISIBLE`] lines.
pub const HELP_PAGES: [(&str, &[&str]); 4] = [
    ("KEYS 1/4", &["A  Z  LEFT", "B  X  ENTER", "C  C  RIGHT", "A+C/M  MUTE"]),
    ("KEYS 2/4", &["F1  HELP", "F2  SPEED", "F3  SHELL"]),
    ("MOUSE 3/4", &["CLICK ABC OR", "AN ICON", "SCREEN   B", "WHEEL  A/BACK"]),
    ("MOUSE 4/4", &["R-CLICK  C", "AT HOME, IT", "OPENS SETTINGS", "DRAG TO MOVE"]),
];

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
            Page::Menu => 11,
            Page::Shell => Part::ALL.len() + 3,
            Page::Part(p) => p.fields().len() + 1,
            Page::Help | Page::Closed => 0,
        }
    }
    /// The page's title.
    pub fn title(&self) -> &'static str {
        match self.page {
            Page::Closed | Page::Menu => "SETTINGS",
            Page::Help => HELP_PAGES[self.help.min(HELP_PAGES.len() - 1)].0,
            Page::Shell => "SHELL",
            Page::Part(p) => p.name(),
        }
    }
    /// The first row on screen: rows show [`VISIBLE`] at a time, a screenful per step.
    pub fn first_row(&self) -> usize {
        self.hover.unwrap_or(0) / VISIBLE * VISIBLE
    }
    /// Commands from button presses since the last call.
    pub fn take_cmds(&mut self) -> Vec<DesktopCmd> {
        std::mem::take(&mut self.cmds)
    }
    pub(crate) fn push_cmd(&mut self, cmd: DesktopCmd) {
        self.cmds.push(cmd);
    }
}

/// Carry out a menu choice. Settings stored in the game are changed here; the rest is
/// returned for the desktop shell.
pub fn activate(core: &mut Core, act: Act) -> Option<DesktopCmd> {
    let login = core.overlay.env.login;
    let s = &mut core.game.settings;
    match act {
        Act::Size => return Some(step_size(core, 1)),
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
        Act::Login => return Some(DesktopCmd::SetLogin(!login)),
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

fn step_size(core: &mut Core, dir: i32) -> DesktopCmd {
    let s = &mut core.game.settings;
    let i = SIZES.iter().position(|x| x.0 == size_name(s.ui.scale)).unwrap_or(1) as i32;
    let next = SIZES[(i + dir).rem_euclid(SIZES.len() as i32) as usize].1;
    s.ui.scale = next;
    DesktopCmd::SetScale(next)
}

/// Left and right on the keyboard: step the highlighted setting back or forward. On/off
/// settings flip either way; rows that open a page or do something ignore it.
pub fn adjust(core: &mut Core, dir: i32) -> Option<DesktopCmd> {
    let hover = core.overlay.hover?;
    let act = items(core).get(hover)?.act;
    let s = &mut core.game.settings;
    match act {
        Act::Field(f) => s.look.step(f, dir),
        Act::Preset => {
            let n = crate::look::PRESETS.len() as i32;
            s.set_shell((s.shell as i32 + dir).rem_euclid(n) as u32);
        }
        Act::Size => return Some(step_size(core, dir)),
        Act::Speed => {
            let k = SPEEDS.iter().position(|&x| x == s.speed).unwrap_or(0) as i32;
            s.speed = SPEEDS[(k + dir).rem_euclid(SPEEDS.len() as i32) as usize];
        }
        Act::Sound | Act::OnTop | Act::Notify | Act::Login => return activate(core, act),
        _ => {}
    }
    None
}

/// A press of one of the device's buttons while the settings are open: A is the next row,
/// B changes or opens it, C goes back a page (and closes the settings from the top).
pub fn button(core: &mut Core, b: Button) -> Option<DesktopCmd> {
    match (core.overlay.page, b) {
        (Page::Closed, _) => None,
        (_, Button::C) => {
            core.play(Tune::Click);
            core.overlay.back();
            None
        }
        (Page::Help, _) => {
            core.play(Tune::Click);
            core.overlay.next_help();
            None
        }
        (_, Button::A) => {
            core.play(Tune::Click);
            scroll(core, 1);
            None
        }
        (_, Button::B) => {
            let act = items(core).get(core.overlay.hover.unwrap_or(0))?.act;
            // the sound row beeps at its new volume by itself
            if act != Act::Sound {
                core.play(Tune::Ok);
            }
            activate(core, act)
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

// ====================================================================== on the screen
/// The settings screen's size in dots: the LCD's, doubled.
pub const GRID_W: i32 = crate::lcd::W * 2;
pub const GRID_H: i32 = crate::lcd::H * 2;
/// Rows on screen at once, under the title.
pub const VISIBLE: usize = 4;
/// Dots from one row to the next, and where the first row's highlight starts.
const ROW_DOTS: i32 = 6;
const FIRST_ROW: i32 = 7;
/// The smallest dot, in window pixels, that keeps the letters readable. On the smallest
/// devices the grid grows past the LCD (over its frame) to keep to it.
pub const MIN_DOT: f32 = 2.0;

/// Where the settings screen's dots sit, in window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub x: f32,
    pub y: f32,
    /// Pixels per dot.
    pub dot: f32,
}

/// The grid for a window of `scale` pixels per design unit: normally exactly over the LCD's
/// dots, two to each of them, centred on the LCD when it has to grow.
pub fn grid(scale: f32) -> Grid {
    let dot = (CELL / 2.0 * scale).max(MIN_DOT);
    let cx = (LCD_POS.0 + crate::lcd::W as f32 * CELL / 2.0 - VIEW.0) * scale;
    let cy = (LCD_POS.1 + crate::lcd::H as f32 * CELL / 2.0 - VIEW.1) * scale;
    Grid { x: (cx - GRID_W as f32 * dot / 2.0).round(), y: (cy - GRID_H as f32 * dot / 2.0).round(), dot }
}

impl Grid {
    pub fn w(&self) -> f32 {
        GRID_W as f32 * self.dot
    }
    pub fn h(&self) -> f32 {
        GRID_H as f32 * self.dot
    }
    /// One dot's left, top, right and bottom edges, on whole pixels like the LCD's dots.
    pub fn edges(&self, gx: i32, gy: i32) -> (f32, f32, f32, f32) {
        let at = |o: f32, i: i32| (o + i as f32 * self.dot).round();
        (at(self.x, gx), at(self.y, gy), at(self.x, gx + 1), at(self.y, gy + 1))
    }
    /// The dot under a window-pixel position.
    pub fn at(&self, px: f32, py: f32) -> Option<(i32, i32)> {
        let gx = ((px - self.x) / self.dot).floor() as i32;
        let gy = ((py - self.y) / self.dot).floor() as i32;
        ((0..GRID_W).contains(&gx) && (0..GRID_H).contains(&gy)).then_some((gx, gy))
    }
}

/// A [`GRID_W`]x[`GRID_H`] bitmap, row by row.
struct Dots(Vec<bool>);

impl Dots {
    fn set(&mut self, x: i32, y: i32, on: bool) {
        if (0..GRID_W).contains(&x) && (0..GRID_H).contains(&y) {
            self.0[(y * GRID_W + x) as usize] = on;
        }
    }
    fn text(&mut self, s: &str, x: i32, y: i32, on: bool) {
        for_each_dot(s, |dx, dy| self.set(x + dx, y + dy, on));
    }
    fn fill(&mut self, y: i32, h: i32) {
        for yy in y..y + h {
            for x in 0..GRID_W {
                self.set(x, yy, true);
            }
        }
    }
}

/// Where a row's text starts, `r` rows down the screen.
fn row_y(r: usize) -> i32 {
    FIRST_ROW + 1 + r as i32 * ROW_DOTS
}

/// The open page as lit dots: the title (with the highlighted row's number), then a screenful
/// of rows with the highlighted one shown inverted, or a help page's lines.
pub fn dots(core: &Core) -> Vec<bool> {
    let o = &core.overlay;
    let mut d = Dots(vec![false; (GRID_W * GRID_H) as usize]);
    if !o.is_open() {
        return d.0;
    }
    d.text(o.title(), 1, 1, true);
    match o.page {
        Page::Closed => {}
        Page::Help => {
            let (_, lines) = HELP_PAGES[o.help.min(HELP_PAGES.len() - 1)];
            for (r, l) in lines.iter().enumerate() {
                d.text(l, 1, row_y(r), true);
            }
        }
        Page::Menu | Page::Shell | Page::Part(_) => {
            let items = items(core);
            let hover = o.hover.unwrap_or(0);
            let count = format!("{}/{}", hover + 1, items.len());
            d.text(&count, GRID_W - 1 - text_width(&count), 1, true);
            let first = o.first_row();
            for (r, it) in items.iter().enumerate().skip(first).take(VISIBLE) {
                let y = row_y(r - first);
                let lit = r == hover;
                if lit {
                    d.fill(y - 1, ROW_DOTS + 1);
                }
                d.text(it.label, 1, y, !lit);
                d.text(&it.value, GRID_W - 1 - text_width(&it.value), y, !lit);
            }
        }
    }
    d.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{icon_rects, BUTTONS, FACE, STAR};

    fn core() -> Core {
        let content = crate::content::Content::builtin();
        let game = crate::sim::Game::new(&content, crate::rng::Pcg32::new(1));
        Core::new(content, game)
    }

    /// Window pixels for a design-space point.
    fn px(scale: f32, x: f32, y: f32) -> (f32, f32) {
        ((x - VIEW.0) * scale, (y - VIEW.1) * scale)
    }

    #[test]
    fn the_screen_stays_on_the_faceplate_and_readable_at_every_size() {
        for &(name, size) in &SIZES {
            for dpi in [1.0f32, 1.25, 1.5, 2.0] {
                let scale = size * dpi;
                let g = grid(scale);
                assert!(g.dot >= MIN_DOT, "{name} @{dpi}: dots of {} px", g.dot);
                // always on the faceplate
                let (fx, fy) = px(scale, FACE.0, FACE.1);
                let (fx1, fy1) = px(scale, FACE.0 + FACE.2, FACE.1 + FACE.3);
                assert!(g.x >= fx && g.y >= fy && g.x + g.w() <= fx1 && g.y + g.h() <= fy1, "{name} @{dpi}: {g:?}");
                // and, unless the LCD had to grow, clear of the icons
                if CELL / 2.0 * scale >= MIN_DOT {
                    for &(x, y, w, h) in &icon_rects() {
                        let (ix, iy) = px(scale, x, y);
                        let (ix1, iy1) = px(scale, x + w, y + h);
                        let apart = ix1 <= g.x || ix >= g.x + g.w() || iy1 <= g.y || iy >= g.y + g.h();
                        assert!(apart, "{name} @{dpi}: the screen covers an icon");
                    }
                }
            }
        }
    }

    #[test]
    fn every_line_fits_the_screen() {
        // a dot's margin each side and at least a space between label and value
        let fits = |a: &str, b: &str| text_width(a) + if b.is_empty() { 0 } else { 5 + text_width(b) } <= GRID_W - 2;
        for hide in ["HIDE TO TRAY", "MINIMIZE"] {
            let mut core = core();
            core.overlay.env = Env { login: false, hide_label: hide };
            let check = |core: &Core| {
                for it in items(core) {
                    assert!(fits(it.label, &it.value), "{} {} is too wide", it.label, it.value);
                }
                let n = items(core).len();
                assert_eq!(n, core.overlay.rows());
                assert!(fits(core.overlay.title(), &format!("{n}/{n}")), "{}", core.overlay.title());
            };
            core.overlay.open();
            for &(_, s) in &SIZES {
                core.game.settings.ui.scale = s;
                check(&core);
            }
            // every preset, and CUSTOM, on the menu and the SHELL page
            for shell in 0..=crate::look::PRESETS.len() as u32 {
                core.game.settings.set_shell(shell);
                if shell as usize == crate::look::PRESETS.len() {
                    core.game.settings.look.step(Field::Ring, 1);
                }
                for page in [Page::Menu, Page::Shell] {
                    core.overlay.page = page;
                    check(&core);
                }
            }
            // every option of every part
            for part in Part::ALL {
                core.overlay.page = Page::Part(part);
                for &f in part.fields() {
                    for i in 0..f.count() {
                        core.game.settings.look.set(f, i);
                        check(&core);
                    }
                }
            }
        }
        for (title, lines) in HELP_PAGES {
            assert!(lines.len() <= VISIBLE, "{title} has too many lines");
            for l in std::iter::once(&title).chain(lines.iter()) {
                assert!(fits(l, ""), "help line {l:?} is too wide");
            }
        }
        // the last row's highlight ends on the screen's last line of dots
        assert_eq!(row_y(VISIBLE - 1) + 5, GRID_H - 1);
    }

    #[test]
    fn the_page_is_drawn_with_the_highlighted_row_inverted() {
        let mut core = core();
        core.overlay.open();
        core.overlay.hover = Some(4);
        let d = dots(&core);
        let lit = |x: i32, y: i32| d[(y * GRID_W + x) as usize];
        // the second screenful, rows 4-7; ON TOP (row 4) is first and lit right across
        let y = row_y(0);
        assert!(lit(0, y - 1) && lit(GRID_W - 1, y + 5));
        assert!(!lit(0, row_y(1)));
        // the title and the row count are lit dots on the unlit screen
        assert!((0..GRID_W).any(|x| lit(x, 1)));
        assert!(lit(GRID_W - 2, 5), "5/11 ends in a 1, whose base is 3 dots wide");
        // nothing is lit while the settings are shut
        core.overlay.close();
        assert!(dots(&core).iter().all(|&v| !v));
    }

    #[test]
    fn the_device_buttons_work_the_settings() {
        let mut core = core();
        core.overlay.open();
        // A walks down the rows, B changes one, C goes back and finally closes
        core.press(Button::A);
        core.release(Button::A);
        assert_eq!(core.overlay.hover, Some(1));
        core.tap(Button::B);
        assert_eq!(core.overlay.page, Page::Shell);
        core.tap(Button::B);
        assert_eq!(core.game.settings.shell_name(), "LIME");
        core.tap(Button::C);
        assert_eq!((core.overlay.page, core.overlay.hover), (Page::Menu, Some(1)));
        // the SIZE row's command waits for the desktop
        core.overlay.hover = Some(0);
        core.tap(Button::B);
        assert!(matches!(core.overlay.take_cmds()[..], [DesktopCmd::SetScale(_)]));
        core.tap(Button::C);
        assert!(!core.overlay.is_open());
        // the game didn't see any of it: still on the home screen with nothing selected
        assert!(core.at_home());
        assert_eq!(core.ui.sel, -1);
    }

    #[test]
    fn the_wheel_picks_a_row_and_a_click_chooses_it() {
        // with the mouse, the wheel moves the highlight and a click is B (see the desktop shell)
        let mut core = core();
        core.overlay.open();
        // three notches down to SOUND, and a click toggles the sound
        let muted = core.game.settings.muted;
        scroll(&mut core, 3);
        core.tap(Button::B);
        assert_ne!(core.game.settings.muted, muted);
        // down to HELP, on the next screenful
        scroll(&mut core, 4);
        assert_eq!((core.overlay.hover, core.overlay.first_row()), (Some(7), 4));
        core.tap(Button::B);
        assert_eq!((core.overlay.page, core.overlay.help), (Page::Help, 0));
        // clicks turn the help pages, then it's back to the menu
        for i in 1..HELP_PAGES.len() {
            core.tap(Button::B);
            assert_eq!((core.overlay.page, core.overlay.help), (Page::Help, i));
        }
        core.tap(Button::B);
        assert_eq!(core.overlay.page, Page::Menu);
        // up past the top wraps round to CLOSE, and choosing it closes the settings
        core.overlay.hover = Some(0);
        scroll(&mut core, -1);
        assert_eq!(items(&core)[core.overlay.hover.unwrap()].label, "CLOSE");
        core.tap(Button::B);
        assert!(!core.overlay.is_open());
    }

    #[test]
    fn the_shell_pages() {
        let mut core = core();
        let scale = 0.55;
        let g = grid(scale);
        let click_row = |core: &mut Core, r: usize| {
            core.overlay.hover = Some(r);
            core.tap(Button::B);
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
        assert_eq!(items(&core)[1].value, "ROUND");
        click_row(&mut core, 1);
        assert_eq!(items(&core)[1].value, "SQUARE");
        assert_eq!(core.game.settings.look.body, crate::look::PRESETS[1].look.body);
        assert_eq!(core.game.settings.shell_name(), "CUSTOM");
        // left and right step either way
        core.overlay.hover = Some(1);
        adjust(&mut core, -1);
        assert_eq!(core.game.settings.shell_name(), "LIME");
        // the screen leaves the buttons and the antenna in sight
        for &(_, bx, by) in &BUTTONS {
            let (x, y) = px(scale, bx, by);
            assert_eq!(g.at(x, y), None, "the screen hides the buttons");
        }
        let (x, y) = px(scale, STAR.0, STAR.1);
        assert_eq!(g.at(x, y), None, "the screen hides the antenna");
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
            activate(&mut core, Act::Random);
            assert_ne!(core.game.settings.look, before);
        }
        // the scroll wheel wraps within the page
        core.overlay.go(Page::Part(Part::Antenna), 0);
        scroll(&mut core, -1);
        assert_eq!(core.overlay.hover, Some(3));
    }

    #[test]
    fn left_and_right_step_sizes_and_speeds_both_ways() {
        let mut core = core();
        core.overlay.open();
        core.game.settings.ui.scale = 0.55;
        assert_eq!(adjust(&mut core, -1), Some(DesktopCmd::SetScale(0.40)));
        assert_eq!(adjust(&mut core, -1), Some(DesktopCmd::SetScale(1.0)));
        core.overlay.hover = Some(2);
        adjust(&mut core, -1);
        assert_eq!(core.game.settings.speed, 600);
        adjust(&mut core, 1);
        assert_eq!(core.game.settings.speed, 1);
        // on/off rows flip; rows that open pages ignore it
        core.overlay.hover = Some(5);
        let notify = core.game.settings.ui.notify;
        adjust(&mut core, -1);
        assert_ne!(core.game.settings.ui.notify, notify);
        core.overlay.hover = Some(1);
        adjust(&mut core, 1);
        assert_eq!(core.overlay.page, Page::Menu);
    }

    #[test]
    fn sound_steps_through_the_volumes() {
        let mut core = core();
        let sound = |core: &Core| items(core)[3].value.clone();
        assert_eq!(sound(&core), "100%");
        let mut seen = Vec::new();
        for _ in 0..5 {
            activate(&mut core, Act::Sound);
            seen.push(sound(&core));
        }
        assert_eq!(seen, ["OFF", "25%", "50%", "75%", "100%"]);
        // muting keeps the volume for when the sound comes back
        core.set_sound(Some(50));
        core.toggle_mute();
        assert_eq!(sound(&core), "OFF");
        core.toggle_mute();
        assert_eq!(sound(&core), "50%");
    }
}
