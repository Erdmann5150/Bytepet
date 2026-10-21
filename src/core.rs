//! The app core: owns the game, the screen stack and the LCD. Front-ends call `frame()` once per
//! frame and forward input; they read `lcd`/`shade`, `icon_states()` and drain `take_sfx()` /
//! `take_notices()`.

use std::collections::VecDeque;
use std::path::PathBuf;

use crate::content::Content;
use crate::lcd::{Lcd, H, W};
use crate::overlay::Overlay;
use crate::rng::Pcg32;
use crate::save;
use crate::screens::{Ctx, DrawCx, Key, MainScreen, Message, Res, Screen, Tr, UiState};
use crate::sim::{CallReason, Event, Game};
use crate::{Button, Tune};

pub const SAVE_EVERY: f64 = 20.0;
/// Seconds without a press before a menu left open goes back to the home screen.
pub const MENU_TIMEOUT: f64 = 60.0;
/// Seconds without a press before the home screen's icon cursor goes out.
pub const SELECT_TIMEOUT: f64 = 30.0;

pub struct Core {
    pub content: Content,
    pub game: Game,
    pub stack: Vec<Box<dyn Screen>>,
    pub lcd: Lcd,
    /// LCD dots fade toward their target, like a real LCD's slow response.
    pub shade: [f32; (W * H) as usize],
    pub ui: UiState,
    pub overlay: Overlay,
    pub held: [bool; 3],
    /// Buttons pressed while the settings were open: their release isn't the game's.
    menu_held: [bool; 3],
    /// A clicked one-shot icon stays lit at least this much longer, then goes out once its
    /// animation is over and the home screen is back.
    unlight: Option<f64>,
    /// Real seconds since the last button, key, click or scroll (see [`MENU_TIMEOUT`]).
    idle: f64,
    pending: VecDeque<Event>,
    sfx: Vec<Tune>,
    notices: Vec<Event>,
    save_path: Option<PathBuf>,
    save_timer: f64,
    /// Wall clock of the previous frame (None until the first frame).
    last_wall: Option<f64>,
    /// Local hour for a unix timestamp (replaceable in tests).
    pub hour_at: fn(f64) -> f64,
    /// Message shown at startup (load problems, time away).
    pub startup_notes: Vec<String>,
}

macro_rules! ctx {
    ($s:ident) => {
        Ctx {
            game: &mut $s.game,
            content: &$s.content,
            ui: &mut $s.ui,
            sfx: &mut $s.sfx,
            held: $s.held,
            pending: &mut $s.pending,
        }
    };
}

pub fn now_unix() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// "MOCHI" as "Mochi".
pub fn title_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i == 0 {
            out.extend(ch.to_uppercase());
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

/// Local hour of day (0..24) for a unix timestamp.
pub fn local_hour(ts: f64) -> f64 {
    use chrono::{Local, TimeZone, Timelike};
    match Local.timestamp_opt(ts as i64, 0) {
        chrono::LocalResult::Single(t) | chrono::LocalResult::Ambiguous(t, _) => {
            t.hour() as f64 + t.minute() as f64 / 60.0 + t.second() as f64 / 3600.0
        }
        chrono::LocalResult::None => 12.0,
    }
}

impl Core {
    /// A core around an existing game (tests, or a freshly loaded save).
    pub fn new(content: Content, game: Game) -> Core {
        Core {
            content,
            game,
            stack: vec![Box::new(MainScreen::new())],
            lcd: Lcd::default(),
            shade: [0.0; (W * H) as usize],
            ui: UiState { sel: -1, ..UiState::default() },
            overlay: Overlay::default(),
            held: [false; 3],
            menu_held: [false; 3],
            unlight: None,
            idle: 0.0,
            pending: VecDeque::new(),
            sfx: Vec::new(),
            notices: Vec::new(),
            save_path: None,
            save_timer: 0.0,
            last_wall: None,
            hour_at: local_hour,
            startup_notes: Vec::new(),
        }
    }

    /// Load (or create) the save at `path`, catching up on time spent closed.
    pub fn open(content: Content, path: PathBuf, fresh: bool) -> Core {
        let mut notes = Vec::new();
        let now = now_unix();
        let mut away = 0.0;
        let mut writable = true;
        let game = if fresh {
            Game::new(&content, Pcg32::from_time())
        } else {
            match save::load_any(&path) {
                Ok(save::Loaded { file: s, restored }) => {
                    if let Some(e) = restored {
                        notes.push(format!("{e}. Restored the previous save from the backup."));
                    }
                    let mut g = s.game;
                    g.repair(&content);
                    let gap = now - s.saved_at;
                    if g.settings.run_away && gap > 5.0 {
                        away = g.catch_up(&content, gap, s.saved_at, &local_hour);
                    }
                    g
                }
                Err(save::LoadError::Missing) => Game::new(&content, Pcg32::from_time()),
                Err(e @ save::LoadError::TooNew(_)) => {
                    // never overwrite a save made by a newer version
                    writable = false;
                    notes.push(format!("{e}. Running without saving."));
                    Game::new(&content, Pcg32::from_time())
                }
                Err(e) => {
                    notes.push(e.to_string());
                    Game::new(&content, Pcg32::from_time())
                }
            }
        };
        let mut core = Core::new(content, game);
        core.save_path = if writable { Some(path) } else { None };
        core.startup_notes = notes;
        if away > 600.0 {
            let (h, m) = ((away / 3600.0) as u32, ((away % 3600.0) / 60.0) as u32);
            core.stack.push(Box::new(Message::new(vec![
                ("WELCOME".into(), "BACK!".into()),
                ("AWAY".into(), format!("{h}H {m}M")),
            ])));
        }
        // events from the catch-up: keep only the latest of each kind
        core.collect_events(true);
        core
    }

    pub fn save_path(&self) -> Option<&PathBuf> {
        self.save_path.as_ref()
    }

    pub fn save(&mut self) {
        if let Some(p) = &self.save_path {
            if let Err(e) = save::save(p, &self.game, now_unix()) {
                eprintln!("bytepet: save failed: {e}");
            }
        }
    }

    // ------------------------------------------------------------------ screen stack
    /// Names of the open screens, bottom first (tests and debugging).
    pub fn stack_names(&self) -> Vec<&'static str> {
        self.stack.iter().map(|s| s.name()).collect()
    }

    pub fn top_name(&self) -> &'static str {
        self.stack.last().map(|s| s.name()).unwrap_or("")
    }

    pub fn wants_text(&self) -> bool {
        self.stack.last().map(|s| s.wants_text()).unwrap_or(false)
    }

    fn with_top(&mut self, f: impl FnOnce(&mut dyn Screen, &mut Ctx) -> Tr) {
        let Some(mut s) = self.stack.pop() else { return };
        let tr = {
            let mut cx = ctx!(self);
            f(s.as_mut(), &mut cx)
        };
        self.stack.push(s);
        self.apply(tr);
    }

    fn apply(&mut self, tr: Tr) {
        match tr {
            Tr::None => {}
            Tr::Push(s) => self.stack.push(s),
            Tr::PushAll(v) => self.stack.extend(v),
            Tr::Pop => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                }
            }
            Tr::PopWith(r) => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                }
                self.with_top(|s, cx| s.result(cx, r));
            }
            Tr::Replace(s) => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                }
                self.stack.push(s);
            }
            Tr::ToMain => self.stack.truncate(1),
            Tr::ToMainThen(v) => {
                self.stack.truncate(1);
                self.stack.extend(v);
            }
        }
    }

    // ------------------------------------------------------------------ input
    /// Someone used the device: the menu and icon timeouts start over. The desktop shell
    /// calls it for input that works the settings directly.
    pub fn touch(&mut self) {
        self.idle = 0.0;
    }

    pub fn press(&mut self, b: Button) {
        self.touch();
        // A and C together toggle the sound, like the real thing
        if (b == Button::A && self.held[2]) || (b == Button::C && self.held[0]) {
            self.held[b.index()] = true;
            self.toggle_mute();
            return;
        }
        self.held[b.index()] = true;
        // the buttons on the home screen move the cursor from where it is
        if self.at_home() {
            self.unlight = None;
        }
        // while the settings are open the buttons work them, not the game
        if self.overlay.is_open() {
            self.menu_held[b.index()] = true;
            if let Some(cmd) = crate::overlay::button(self, b) {
                self.overlay.push_cmd(cmd);
            }
            return;
        }
        self.with_top(|s, cx| s.press(cx, b));
    }

    pub fn release(&mut self, b: Button) {
        if std::mem::take(&mut self.menu_held[b.index()]) {
            self.held[b.index()] = false;
        } else if self.held[b.index()] {
            self.held[b.index()] = false;
            self.with_top(|s, cx| s.release(cx, b));
        }
    }

    pub fn text(&mut self, s: &str) {
        self.touch();
        self.with_top(|sc, cx| sc.text(cx, s));
    }

    pub fn key(&mut self, k: Key) {
        self.touch();
        self.with_top(|sc, cx| sc.key(cx, k));
    }

    /// Click one of the nine icons. From a menu this goes straight to the other menu, as
    /// long as every screen open above the main one can be left (see `Screen::leavable`).
    /// One-shot icons (clean, lights, medicine) light up only while they act.
    pub fn click_icon(&mut self, i: usize) {
        self.touch();
        if i >= 9 || self.stack.is_empty() || !self.stack[1..].iter().all(|s| s.leavable()) {
            return;
        }
        self.stack.truncate(1);
        self.ui.sel = i as i32;
        self.unlight = crate::screens::main_screen::ONE_SHOT.contains(&i).then_some(0.4);
        self.with_top(|s, cx| s.activate_icon(cx, i));
    }

    /// On the main screen with nothing else open.
    pub fn at_home(&self) -> bool {
        self.stack.len() == 1
    }

    /// Right mouse button: goes up a page of the settings menu if it's open (closing it from the
    /// top), otherwise C. It never opens the settings: that's Town > SETUP > MORE, or the tray.
    pub fn right_click(&mut self) {
        self.touch();
        if self.overlay.is_open() {
            self.overlay.back();
        } else {
            self.tap(Button::C);
        }
    }

    /// Scroll wheel: down is A (next), up moves the menu back one. Where there's no menu to
    /// move back through (the mini-games), up is A too.
    pub fn scroll(&mut self, up: bool) {
        self.touch();
        if up {
            let mut moved = false;
            self.with_top(|s, cx| {
                let tr = s.prev(cx);
                moved = tr.is_some();
                tr.unwrap_or(Tr::None)
            });
            if moved {
                return;
            }
        }
        self.tap(Button::A);
    }

    /// A quick press and release, for mouse clicks.
    pub fn tap(&mut self, b: Button) {
        self.press(b);
        self.release(b);
    }

    pub fn play(&mut self, t: Tune) {
        self.sfx.push(t);
    }

    pub fn toggle_mute(&mut self) {
        let s = &mut self.game.settings;
        s.muted = !s.muted;
        if !s.muted {
            self.sfx.push(Tune::Ok);
        }
    }

    /// `None` mutes; a volume unmutes at that level and beeps so you hear how loud it is.
    pub fn set_sound(&mut self, sound: Option<u32>) {
        self.game.settings.set_sound(sound);
        if sound.is_some() {
            self.sfx.push(Tune::Ok);
        }
    }

    /// Off, then each volume up to the loudest, then off again.
    pub fn cycle_sound(&mut self) {
        let next = self.game.settings.next_sound();
        self.set_sound(next);
    }

    pub fn cycle_speed(&mut self) {
        use crate::screens::town::SPEEDS;
        let s = &mut self.game.settings;
        let k = SPEEDS.iter().position(|&x| x == s.speed).unwrap_or(0);
        s.speed = SPEEDS[(k + 1) % SPEEDS.len()];
    }

    pub fn next_shell(&mut self) {
        self.game.settings.next_shell();
    }

    // ------------------------------------------------------------------ per frame
    /// Advance by `dt` real seconds.
    pub fn frame(&mut self, dt: f64) {
        let now = now_unix();
        self.frame_at(dt, now);
    }

    /// `frame` with an explicit wall clock (tests).
    pub fn frame_at(&mut self, dt: f64, now: f64) {
        let dt = dt.clamp(0.0, 0.25);
        let gap = self.last_wall.map(|w| now - w).unwrap_or(0.0);
        self.last_wall = Some(now);
        if gap > 5.0 {
            // the computer slept, or the app was frozen
            if self.game.settings.run_away {
                let hour_at = self.hour_at;
                self.game.catch_up(&self.content, gap, now - gap, &hour_at);
                self.collect_events(true);
            }
        } else {
            let hour = (self.hour_at)(now);
            let mut sim_dt = dt * self.game.settings.speed as f64;
            while sim_dt > 0.0 {
                let step = sim_dt.min(30.0);
                self.game.step(&self.content, step, hour);
                sim_dt -= step;
            }
            self.collect_events(false);
        }
        self.with_top(|s, cx| s.update(cx, dt));
        let n = self.stack.len();
        if n >= 2 && self.stack[n - 1].see_through() {
            self.stack[n - 2].animate(dt);
        }
        if let Some(t) = self.unlight.as_mut() {
            *t -= dt;
            if *t <= 0.0 && self.at_home() {
                self.unlight = None;
                self.ui.sel = -1;
            }
        }
        self.idle_out(dt);
        self.render_lcd();
        let k = 1.0 - (-dt as f32 * 18.0).exp();
        for (sh, &v) in self.shade.iter_mut().zip(self.lcd.buf.iter()) {
            *sh += (v as f32 - *sh) * k;
            // settle exactly once the difference can't be seen, so a still screen stays still
            // (and the desktop shell can skip redrawing it)
            if (v as f32 - *sh).abs() < 1.0 / 512.0 {
                *sh = v as f32;
            }
        }
        self.save_timer += dt;
        if self.save_timer >= SAVE_EVERY {
            self.save_timer = 0.0;
            self.save();
        }
    }

    /// Left alone, the device goes back to its home screen like the real thing: an open menu
    /// (or the settings) after [`MENU_TIMEOUT`], then the icon cursor after [`SELECT_TIMEOUT`].
    /// Only menus that could be left by clicking an icon time out (see `Screen::leavable`), so
    /// a game, an animation or a link is never cut short.
    fn idle_out(&mut self, dt: f64) {
        self.idle += dt;
        if self.idle >= MENU_TIMEOUT {
            if self.overlay.is_open() {
                self.overlay.close();
            }
            if !self.at_home() && self.stack[1..].iter().all(|s| s.leavable()) {
                self.stack.truncate(1);
            }
        }
        if self.idle >= SELECT_TIMEOUT && self.at_home() && self.ui.sel >= 0 && self.unlight.is_none() {
            self.ui.sel = -1;
        }
    }

    fn collect_events(&mut self, summarize: bool) {
        let mut evs: Vec<Event> = std::mem::take(&mut self.game.events);
        if summarize {
            // after a long gap only the latest of each kind matters
            let mut seen = Vec::new();
            let mut keep = Vec::new();
            for e in evs.into_iter().rev() {
                let kind = std::mem::discriminant(&e);
                if !seen.contains(&kind) && !matches!(e, Event::Poop | Event::Mistake | Event::Sleep | Event::Wake) {
                    seen.push(kind);
                    keep.push(e);
                }
            }
            keep.reverse();
            evs = keep;
        }
        for e in evs {
            self.notices.push(e.clone());
            if !matches!(e, Event::Poop | Event::Mistake | Event::Sleep | Event::Wake | Event::NewEgg) {
                self.pending.push_back(e);
            }
        }
        while self.pending.len() > 12 {
            self.pending.pop_front();
        }
    }

    pub fn render_lcd(&mut self) {
        self.lcd.clear(0);
        if let Some(top) = self.stack.last() {
            let dcx = DrawCx { game: &self.game, content: &self.content, ui: &self.ui };
            // a see-through screen draws over the one below
            if top.see_through() && self.stack.len() >= 2 {
                self.stack[self.stack.len() - 2].draw(&dcx, &mut self.lcd);
            }
            top.draw(&dcx, &mut self.lcd);
        }
    }

    pub fn take_sfx(&mut self) -> Vec<Tune> {
        std::mem::take(&mut self.sfx)
    }

    pub fn take_notices(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.notices)
    }

    /// Which of the ten bezel icons are dark right now.
    pub fn icon_states(&self, wall: f64) -> [bool; 10] {
        let p = &self.game.pet;
        let blink = (wall * 2.0) as i64 % 2 == 1;
        let mut out = [false; 10];
        for (i, o) in out.iter_mut().enumerate() {
            *o = i as i32 == self.ui.sel;
        }
        out[9] = p.needs_attention();
        if !p.mail.is_empty() && blink {
            out[8] = true;
        }
        out
    }

    /// How the pet is doing in a line or two, for the tray icon's tooltip: who it is, its
    /// hearts, and what it needs (the same things that light the alert icon). Kept well under
    /// the 127 characters Windows allows.
    pub fn summary(&self) -> String {
        use crate::content::Stage;
        let p = &self.game.pet;
        let name = title_case(&p.display_name(&self.content));
        let stage = match p.stage {
            Stage::Egg => return "An egg, waiting to hatch".into(),
            Stage::Gone => return format!("{name} went home. A new egg is waiting."),
            Stage::Baby => "baby",
            Stage::Child => "child",
            Stage::Teen => "teen",
            Stage::Adult => "adult",
            Stage::Senior => "senior",
        };
        let days = p.age_days();
        let age = if days == 1 { "1 day".to_string() } else { format!("{days} days") };
        let head = format!("{name} the {stage}, {age} - hunger {}/4, happy {}/4", p.hunger, p.happy);
        let mut needs = Vec::new();
        if !p.asleep && p.hunger == 0 {
            needs.push("food");
        }
        if !p.asleep && p.happy == 0 {
            needs.push("fun");
        }
        if p.poop > 0 {
            needs.push("cleaning");
        }
        if p.sick {
            needs.push("medicine");
        }
        // only while it's calling about them, like the alert icon (see `Pet::needs_attention`)
        match p.call.as_ref().map(|c| c.reason) {
            Some(CallReason::Lights) => needs.push("lights off"),
            Some(CallReason::Fake) => needs.push("discipline"),
            _ => {}
        }
        let tail = match (needs.is_empty(), p.asleep) {
            (false, _) => format!("Needs {}", needs.join(", ")),
            (true, true) => "Asleep".into(),
            (true, false) => "All good".into(),
        };
        format!("{head}\n{tail}")
    }

    pub fn status_tags(&self) -> String {
        let s = &self.game.settings;
        let mut t = Vec::new();
        if s.speed != 1 {
            t.push(format!("{}X", s.speed));
        }
        if s.muted {
            t.push("MUTE".into());
        }
        t.join(" ")
    }

    /// Deliver a result to the top screen directly (used by the link screen's network events).
    pub fn deliver(&mut self, r: Res) {
        self.with_top(|s, cx| s.result(cx, r));
    }
}
