//! The pet simulation. Pure logic: no clock, no IO.
//!
//! `Game::step(content, dt, hour)` advances the pet by `dt` seconds of pet time; `hour` is the
//! local time of day (0..24), which only drives the sleep schedule. The time-speed setting
//! multiplies `dt` but never touches the clock, so bedtime always matches the real day. The
//! bedtime shift moves the whole schedule by whole hours of that real day, for night-shift
//! players, and is the only thing that can.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::content::{Content, Skill, Stage};
use crate::rng::Pcg32;

pub const DAY: f64 = 86400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallReason {
    Need,
    Lights,
    Fake,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub reason: CallReason,
    pub t: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailKind {
    Gp,
    Heart,
    Trick,
    Item,
    School,
    Job,
    Love,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mail {
    pub kind: MailKind,
    pub t: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Skills {
    pub humor: u32,
    pub gorgeous: u32,
    pub passion: u32,
}

impl Skills {
    pub fn get(&self, s: Skill) -> u32 {
        match s {
            Skill::Humor => self.humor,
            Skill::Gorgeous => self.gorgeous,
            Skill::Passion => self.passion,
        }
    }
    pub fn add(&mut self, s: Skill, v: u32) {
        let slot = match s {
            Skill::Humor => &mut self.humor,
            Skill::Gorgeous => &mut self.gorgeous,
            Skill::Passion => &mut self.passion,
        };
        *slot = slot.saturating_add(v);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Pet {
    /// Unique per pet; screens use it to ignore results meant for a pet that has since left.
    pub id: u64,
    pub name: String,
    pub gen: u32,
    pub stage: Stage,
    pub character: String,
    pub t: f64,
    pub age_s: f64,
    pub stage_s: f64,
    pub hunger: u8,
    pub happy: u8,
    pub weight: i32,
    pub discipline: u8,
    pub poop: u8,
    pub sick: bool,
    pub doses: u8,
    pub asleep: bool,
    pub lights: bool,
    pub mistakes: u32,
    pub baby_mistakes: u32,
    pub skills: Skills,
    pub job: Option<String>,
    pub last_work: f64,
    pub married: bool,
    pub partner: Option<String>,
    pub mystic: bool,
    pub call: Option<Call>,
    pub need_called: bool,
    pub recall_t: f64,
    pub scold_t: f64,
    pub praise_t: f64,
    pub t_hunger: f64,
    pub t_happy: f64,
    pub t_poop: f64,
    pub t_mail: f64,
    pub t_fake: f64,
    pub t_show: f64,
    pub zero_s: f64,
    pub poop_s: f64,
    pub sick_s: f64,
    pub fat_s: f64,
    pub mail: Vec<Mail>,
    pub last_school_day: i64,
    pub gone_reason: Option<String>,
}

impl Default for Pet {
    fn default() -> Self {
        Pet {
            id: 0,
            name: String::new(),
            gen: 1,
            stage: Stage::Egg,
            character: "egg".into(),
            t: 0.0,
            age_s: 0.0,
            stage_s: 0.0,
            hunger: 0,
            happy: 0,
            weight: 5,
            discipline: 0,
            poop: 0,
            sick: false,
            doses: 0,
            asleep: false,
            lights: true,
            mistakes: 0,
            baby_mistakes: 0,
            skills: Skills::default(),
            job: None,
            last_work: -1e12,
            married: false,
            partner: None,
            mystic: false,
            call: None,
            need_called: false,
            recall_t: 0.0,
            scold_t: 0.0,
            praise_t: 0.0,
            t_hunger: 0.0,
            t_happy: 0.0,
            t_poop: 0.0,
            t_mail: 3600.0,
            t_fake: 14400.0,
            t_show: 18000.0,
            zero_s: 0.0,
            poop_s: 0.0,
            sick_s: 0.0,
            fat_s: 0.0,
            mail: Vec::new(),
            last_school_day: -1,
            gone_reason: None,
        }
    }
}

impl Pet {
    pub fn age_days(&self) -> u32 {
        (self.age_s / DAY) as u32
    }
    pub fn alive(&self) -> bool {
        !matches!(self.stage, Stage::Egg | Stage::Gone)
    }
    pub fn awake(&self) -> bool {
        self.alive() && !self.asleep
    }
    pub fn display_name(&self, c: &Content) -> String {
        if self.name.is_empty() {
            c.name_of(&self.character)
        } else {
            self.name.clone()
        }
    }
    /// Something is waiting to be done for the pet: a call, poop to clean up, a sickness, an
    /// empty heart, or the lights left on while it sleeps. The alert icon shows it. Hearts
    /// can't be filled while it sleeps, so they don't count then.
    pub fn needs_attention(&self) -> bool {
        if !self.alive() {
            return false;
        }
        let hearts = !self.asleep && (self.hunger == 0 || self.happy == 0);
        let lights = self.asleep && self.lights;
        self.call.is_some() || self.poop > 0 || self.sick || hearts || lights
    }
    pub fn top_skill_value(&self) -> u32 {
        Skill::ALL.iter().map(|&s| self.skills.get(s)).max().unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AlbumEntry {
    pub name: String,
    pub character: String,
    pub gen: u32,
    pub age: u32,
    pub job: Option<String>,
    pub partner: Option<String>,
    pub ending: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Friend {
    pub name: String,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub scale: f32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub on_top: bool,
    pub notify: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        UiSettings { scale: 0.55, x: None, y: None, on_top: true, notify: true }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub speed: u32,
    pub muted: bool,
    /// Loudness in percent while not muted; one of [`VOLUMES`] unless a save says otherwise.
    pub volume: u32,
    /// The preset in [`crate::look::PRESETS`] last chosen; `look` may have changed since.
    pub shell: u32,
    pub look: crate::look::Look,
    /// Keep living while the app is closed.
    pub run_away: bool,
    /// Looks the player saved, numbered from 1 in the menus (THEME 1, THEME 2...).
    pub themes: Vec<crate::look::Look>,
    /// Hours the pet's bedtime and waking time move by, for players who sleep by day (see
    /// [`Settings::pet_hour`]). It is a fixed offset from the real clock, so the time speed
    /// still never moves bedtime.
    pub bedtime_shift: i32,
    pub ui: UiSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            speed: 1,
            muted: false,
            volume: 100,
            shell: 0,
            look: crate::look::Look::default(),
            run_away: true,
            themes: Vec::new(),
            bedtime_shift: 0,
            ui: UiSettings::default(),
        }
    }
}

/// The volume steps the menus offer, in percent.
pub const VOLUMES: [u32; 4] = [25, 50, 75, 100];
/// How many looks can be saved as themes.
pub const MAX_THEMES: usize = 9;
/// The bedtime shifts on offer, in hours: a whole day's worth, earlier or later.
pub const BEDTIME_SHIFTS: std::ops::RangeInclusive<i32> = -11..=12;

impl Settings {
    /// The sound as the menus show it: `None` when muted, else the volume.
    pub fn sound(&self) -> Option<u32> {
        (!self.muted).then_some(self.volume)
    }

    /// `None` mutes; a volume unmutes at that level.
    pub fn set_sound(&mut self, sound: Option<u32>) {
        match sound {
            None => self.muted = true,
            Some(v) => {
                self.muted = false;
                self.volume = v.min(100);
            }
        }
    }

    /// What a SOUND row steps to: off, then each volume up to the loudest, then off again.
    pub fn next_sound(&self) -> Option<u32> {
        match self.sound() {
            None => Some(VOLUMES[0]),
            Some(v) => VOLUMES.iter().copied().find(|&x| x > v),
        }
    }

    /// Switch to the next preset look.
    pub fn next_shell(&mut self) {
        self.set_shell(self.shell + 1);
    }

    pub fn set_shell(&mut self, i: u32) {
        use crate::look::PRESETS;
        self.shell = i % PRESETS.len() as u32;
        self.look = PRESETS[self.shell as usize].look;
    }

    /// The preset's name, or CUSTOM once any part has been changed.
    pub fn shell_name(&self) -> &'static str {
        crate::look::preset_name(&self.look, self.shell)
    }

    /// "OFF" or the volume, e.g. "75%".
    pub fn sound_label(&self) -> String {
        self.sound().map_or("OFF".into(), |v| format!("{v}%"))
    }

    /// The saved theme the device is wearing, if any.
    pub fn theme(&self) -> Option<usize> {
        self.themes.iter().position(|t| *t == self.look)
    }

    /// [`Settings::shell_name`], or the saved theme a custom look is, e.g. "THEME 2".
    pub fn shell_label(&self) -> String {
        match (self.shell_name(), self.theme()) {
            ("CUSTOM", Some(i)) => format!("THEME {}", i + 1),
            (name, _) => name.into(),
        }
    }

    /// Changing the look would lose it: it is neither a preset nor a saved theme.
    pub fn look_unsaved(&self) -> bool {
        self.shell_name() == "CUSTOM" && self.theme().is_none()
    }

    /// Save the look as a theme; its index, or `None` when every slot is taken.
    pub fn save_theme(&mut self) -> Option<usize> {
        if let Some(i) = self.theme() {
            return Some(i);
        }
        if self.themes.len() >= MAX_THEMES {
            return None;
        }
        self.themes.push(self.look);
        Some(self.themes.len() - 1)
    }

    pub fn wear_theme(&mut self, i: usize) {
        if let Some(t) = self.themes.get(i) {
            self.look = *t;
        }
    }

    /// The device keeps wearing the look; it just isn't saved any more.
    pub fn delete_theme(&mut self, i: usize) {
        if i < self.themes.len() {
            self.themes.remove(i);
        }
    }

    /// The hour of the pet's own day for a local hour: bedtime comes `bedtime_shift` hours
    /// later than the content says (earlier when negative).
    pub fn pet_hour(&self, hour: f64) -> f64 {
        (hour - self.bedtime_shift as f64).rem_euclid(24.0)
    }

    /// An hour later, round to the earliest after the latest.
    pub fn next_bedtime_shift(&mut self) {
        let s = self.bedtime_shift + 1;
        self.bedtime_shift = if BEDTIME_SHIFTS.contains(&s) { s } else { *BEDTIME_SHIFTS.start() };
    }

    /// "+0H", "+8H", "-3H".
    pub fn bedtime_label(&self) -> String {
        format!("{:+}H", self.bedtime_shift)
    }
}

/// Things that happened during a step; the UI turns them into animations, beeps and notifications.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Call(CallReason),
    Sleep,
    Wake,
    Hatch,
    Evolve(String),
    Mistake,
    Sick,
    Gone,
    Poop,
    Show,
    Mail,
    NewEgg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feed {
    Ok,
    Full,
    Refuse,
    Asleep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medicine {
    NotSick,
    Again,
    Cured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discipline {
    Good,
    Mean,
    Spoiled,
    Nothing,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gains {
    pub skill: Option<(Skill, u32)>,
    pub gp: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Game {
    pub device_id: String,
    pub pet: Pet,
    pub gp: i64,
    pub items: BTreeMap<String, u32>,
    pub album: Vec<AlbumEntry>,
    pub friends: BTreeMap<String, Friend>,
    pub settings: Settings,
    pub rng: Pcg32,
    pub next_pet_id: u64,
    #[serde(skip)]
    pub events: Vec<Event>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            device_id: String::new(),
            pet: Pet::default(),
            gp: 100,
            items: BTreeMap::new(),
            album: Vec::new(),
            friends: BTreeMap::new(),
            settings: Settings::default(),
            rng: Pcg32::default(),
            next_pet_id: 1,
            events: Vec::new(),
        }
    }
}

pub fn in_window(hour: f64, win: [f64; 2]) -> bool {
    let (start, end) = (win[0], win[1]);
    if start > end {
        hour >= start || hour < end
    } else {
        hour >= start && hour < end
    }
}

impl Game {
    pub fn new(c: &Content, rng: Pcg32) -> Game {
        let mut g = Game { rng, gp: c.rules.life.start_gp, ..Game::default() };
        g.device_id = format!("{:012x}", g.rng.next_u64() & 0xffff_ffff_ffff);
        g.pet = g.fresh_pet(c, 1, false, None);
        g
    }

    /// Called after loading: fill in anything an older or hand-edited save left out.
    pub fn repair(&mut self, c: &Content) {
        if self.device_id.is_empty() {
            self.device_id = format!("{:012x}", self.rng.next_u64() & 0xffff_ffff_ffff);
        }
        if self.pet.id == 0 {
            self.pet.id = self.next_pet_id.max(1);
            self.next_pet_id = self.pet.id + 1;
        }
        if self.pet.stage != Stage::Egg && self.pet.stage != Stage::Gone && c.character(&self.pet.character).is_none() {
            // a mod removed this character: keep the pet, show the closest match
            let path = Some(self.top_skill_peek());
            self.pet.character = c
                .find(self.pet.stage, path, None)
                .or_else(|| c.find(self.pet.stage, None, None))
                .unwrap_or("baby")
                .to_string();
        }
        self.items.retain(|k, v| *v > 0 && c.item(k).is_some());
        if let Some(j) = &self.pet.job {
            if c.job(j).is_none() {
                self.pet.job = None;
            }
        }
        if ![1, 10, 60, 600].contains(&self.settings.speed) {
            self.settings.speed = 1;
        }
        let s = &mut self.settings;
        s.bedtime_shift = s.bedtime_shift.clamp(*BEDTIME_SHIFTS.start(), *BEDTIME_SHIFTS.end());
        s.themes.truncate(MAX_THEMES);
    }

    fn fresh_pet(&mut self, c: &Content, gen: u32, mystic: bool, legacy: Option<&Skills>) -> Pet {
        let id = self.next_pet_id;
        self.next_pet_id += 1;
        let care = &c.rules.care;
        let mut p = Pet { id, gen, mystic, ..Pet::default() };
        p.t_mail = self.rng.uniform(1800.0, 5400.0);
        p.t_fake = self.rng.uniform(care.fake_call_min, care.fake_call_max);
        p.t_show = self.rng.uniform(care.show_off_min, care.show_off_max);
        if let Some(l) = legacy {
            let share = care.legacy_skill_share;
            p.skills = Skills {
                humor: (l.humor as f64 * share) as u32,
                gorgeous: (l.gorgeous as f64 * share) as u32,
                passion: (l.passion as f64 * share) as u32,
            };
        }
        p
    }

    fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    // ================================================================== main tick
    pub fn step(&mut self, c: &Content, dt: f64, hour: f64) {
        if self.pet.stage == Stage::Gone || dt <= 0.0 {
            return;
        }
        let hour = self.settings.pet_hour(hour);
        self.pet.t += dt;
        self.pet.stage_s += dt;
        if self.pet.stage == Stage::Egg {
            if self.pet.stage_s >= c.rules.life.egg {
                self.hatch(c);
            }
            return;
        }
        let care = c.rules.care.clone();
        {
            let p = &mut self.pet;
            p.age_s += dt;
            p.scold_t = (p.scold_t - dt).max(0.0);
            p.praise_t = (p.praise_t - dt).max(0.0);
        }

        // sickness progresses awake or asleep
        if self.pet.sick {
            self.pet.sick_s += dt;
            let limit = if self.pet.stage == Stage::Baby { care.sick_to_gone_baby } else { care.sick_to_gone };
            if self.pet.sick_s >= limit {
                self.go_home(c, "SICK");
                return;
            }
        }

        // sleep schedule (real time of day)
        let rule = c.stage_rule(self.pet.stage).clone();
        let should_sleep = rule.sleep.map(|w| in_window(hour, w)).unwrap_or(false);
        if should_sleep && !self.pet.asleep {
            let p = &mut self.pet;
            p.asleep = true;
            p.praise_t = 0.0;
            p.scold_t = 0.0;
            if p.lights {
                p.call = Some(Call { reason: CallReason::Lights, t: p.t });
                self.emit(Event::Call(CallReason::Lights));
            } else {
                p.call = None;
            }
            self.emit(Event::Sleep);
        } else if !should_sleep && self.pet.asleep {
            let p = &mut self.pet;
            p.asleep = false;
            p.lights = true;
            if matches!(&p.call, Some(Call { reason: CallReason::Lights, .. })) {
                p.call = None;
            }
            self.emit(Event::Wake);
        }

        if self.pet.asleep {
            if let Some(call) = &self.pet.call {
                if call.reason == CallReason::Lights && self.pet.t - call.t >= care.call_timeout {
                    self.mistake();
                    self.pet.call = None;
                }
            }
            return;
        }

        if self.check_evolution(c) {
            return;
        }
        let stage = self.pet.stage;

        // hearts drain
        {
            let p = &mut self.pet;
            p.t_hunger -= dt;
            while p.t_hunger <= 0.0 {
                p.hunger = p.hunger.saturating_sub(1);
                p.t_hunger += rule.hunger;
            }
            p.t_happy -= dt;
            while p.t_happy <= 0.0 {
                p.happy = p.happy.saturating_sub(1);
                p.t_happy += rule.happy;
            }
        }

        let empty = self.pet.hunger == 0 || self.pet.happy == 0;
        if empty {
            let p = &mut self.pet;
            p.zero_s += dt;
            if p.need_called && p.call.is_none() {
                p.recall_t -= dt;
                if p.recall_t <= 0.0 {
                    p.need_called = false;
                }
            }
            let can_call = matches!(&p.call, None | Some(Call { reason: CallReason::Fake, .. }));
            if !p.need_called && can_call {
                p.call = Some(Call { reason: CallReason::Need, t: p.t });
                p.need_called = true;
                self.emit(Event::Call(CallReason::Need));
            }
            if self.pet.zero_s >= care.empty_to_sick && !self.pet.sick {
                self.get_sick();
            }
        } else {
            self.after_care();
        }

        // unanswered calls
        if let Some(call) = self.pet.call.clone() {
            if self.pet.t - call.t >= care.call_timeout {
                if call.reason == CallReason::Need {
                    self.mistake();
                    self.pet.recall_t = if stage == Stage::Baby { care.recall_baby } else { care.recall };
                }
                self.pet.call = None;
            }
        }

        // poop
        self.pet.t_poop -= dt;
        while self.pet.t_poop <= 0.0 {
            if self.pet.poop < 4 {
                self.pet.poop += 1;
                self.emit(Event::Poop);
            }
            self.pet.t_poop += self.rng.uniform(rule.poop[0], rule.poop[1]);
        }
        if self.pet.poop > 0 {
            self.pet.poop_s += dt;
            let limit = if stage == Stage::Baby {
                care.poop_to_sick_baby
            } else if self.pet.poop >= 4 {
                care.poop_to_sick_full
            } else {
                care.poop_to_sick
            };
            if self.pet.poop_s >= limit && !self.pet.sick {
                self.get_sick();
                self.pet.poop_s = 0.0;
            }
        } else {
            self.pet.poop_s = 0.0;
        }

        // overweight
        if self.pet.weight > rule.weight + care.overweight_margin {
            self.pet.fat_s += dt;
            if self.pet.fat_s >= care.overweight_to_sick && !self.pet.sick {
                self.get_sick();
                self.pet.fat_s = 0.0;
            }
        } else {
            self.pet.fat_s = 0.0;
        }

        if stage == Stage::Baby {
            return;
        }

        // discipline: pretend calls and showing off
        if self.pet.discipline < 100 {
            self.pet.t_fake -= dt;
            if self.pet.t_fake <= 0.0 {
                self.pet.t_fake = self.rng.uniform(care.fake_call_min, care.fake_call_max);
                if self.pet.call.is_none() && !empty {
                    self.pet.call = Some(Call { reason: CallReason::Fake, t: self.pet.t });
                    self.emit(Event::Call(CallReason::Fake));
                }
            }
        }
        self.pet.t_show -= dt;
        if self.pet.t_show <= 0.0 {
            self.pet.t_show = self.rng.uniform(care.show_off_min, care.show_off_max);
            self.pet.praise_t = care.show_off_window;
            self.emit(Event::Show);
        }

        // mail
        self.pet.t_mail -= dt;
        if self.pet.t_mail <= 0.0 {
            self.pet.t_mail = self.rng.uniform(care.mail_min, care.mail_max);
            self.deliver_mail(c);
        }
    }

    // ================================================================== life events
    fn hatch(&mut self, c: &Content) {
        let rule = c.stage_rule(Stage::Baby).clone();
        let p = &mut self.pet;
        p.stage = Stage::Baby;
        p.character = c.find(Stage::Baby, None, None).unwrap_or("baby").to_string();
        p.stage_s = 0.0;
        p.hunger = 1;
        p.happy = 1;
        p.weight = rule.weight;
        p.t_hunger = rule.hunger;
        p.t_happy = rule.happy;
        p.t_poop = self.rng.uniform(rule.poop[0], rule.poop[1]);
        self.emit(Event::Hatch);
    }

    fn evolve(&mut self, c: &Content, stage: Stage, character: String) {
        let rule = c.stage_rule(stage).clone();
        let p = &mut self.pet;
        p.stage = stage;
        p.character = character.clone();
        p.stage_s = 0.0;
        p.weight = p.weight.max(rule.weight);
        p.t_hunger = if p.t_hunger > 0.0 { p.t_hunger.min(rule.hunger) } else { rule.hunger };
        p.t_happy = if p.t_happy > 0.0 { p.t_happy.min(rule.happy) } else { rule.happy };
        p.t_poop = self.rng.uniform(rule.poop[0], rule.poop[1]);
        self.emit(Event::Evolve(character));
    }

    fn top_skill_peek(&self) -> Skill {
        let s = &self.pet.skills;
        let best = Skill::ALL.iter().map(|&k| s.get(k)).max().unwrap_or(0);
        *Skill::ALL.iter().find(|&&k| s.get(k) == best).unwrap_or(&Skill::Humor)
    }

    /// Highest skill; ties are broken at random.
    pub fn top_skill(&mut self) -> Skill {
        let s = &self.pet.skills;
        let best = Skill::ALL.iter().map(|&k| s.get(k)).max().unwrap_or(0);
        let tops: Vec<Skill> = Skill::ALL.iter().copied().filter(|&k| s.get(k) == best).collect();
        *self.rng.pick(&tops).unwrap_or(&Skill::Humor)
    }

    fn check_evolution(&mut self, c: &Content) -> bool {
        let life = &c.rules.life;
        let p = &self.pet;
        match p.stage {
            Stage::Baby if p.stage_s >= life.baby => {
                let grade = if p.baby_mistakes <= c.rules.evolution.baby_good_mistakes { "a" } else { "b" };
                let id = c.find(Stage::Child, None, Some(grade)).unwrap_or("child_a").to_string();
                self.evolve(c, Stage::Child, id);
                true
            }
            Stage::Child if p.stage_s >= life.child => {
                let path = self.top_skill();
                let id = c.find(Stage::Teen, Some(path), None).unwrap_or("teen_h").to_string();
                self.evolve(c, Stage::Teen, id);
                true
            }
            Stage::Teen if p.stage_s >= life.teen => {
                let id = self.adult_for(c);
                self.evolve(c, Stage::Adult, id);
                true
            }
            Stage::Adult if !p.married && p.age_days() >= life.senior_age_days => {
                let id = c.find(Stage::Senior, None, None).unwrap_or("senior").to_string();
                self.evolve(c, Stage::Senior, id);
                true
            }
            _ => false,
        }
    }

    pub fn adult_for(&mut self, c: &Content) -> String {
        if self.pet.mystic {
            return c.find(Stage::Adult, None, Some("rare")).unwrap_or("s_m").to_string();
        }
        let ev = &c.rules.evolution;
        let path = self.top_skill();
        let m = self.pet.mistakes.saturating_sub(self.pet.baby_mistakes);
        let grade = if self.pet.skills.get(path) >= ev.secret_skill && m == 0 {
            "secret"
        } else if m <= ev.best_mistakes && self.pet.discipline >= ev.best_discipline {
            "a"
        } else if m <= ev.good_mistakes {
            "b"
        } else if m <= ev.fair_mistakes {
            "c"
        } else {
            "d"
        };
        c.find(Stage::Adult, Some(path), Some(grade)).unwrap_or("h_d").to_string()
    }

    fn mistake(&mut self) {
        self.pet.mistakes += 1;
        if self.pet.stage == Stage::Baby {
            self.pet.baby_mistakes += 1;
        }
        self.emit(Event::Mistake);
    }

    fn get_sick(&mut self) {
        self.pet.sick = true;
        self.pet.sick_s = 0.0;
        self.pet.doses = if self.pet.stage == Stage::Baby { 1 } else { 1 + self.rng.below(2) as u8 };
        self.emit(Event::Sick);
    }

    fn go_home(&mut self, c: &Content, reason: &str) {
        self.record_album(c, "WENT HOME");
        self.pet.stage = Stage::Gone;
        self.pet.gone_reason = Some(reason.into());
        self.pet.call = None;
        self.emit(Event::Gone);
    }

    pub fn record_album(&mut self, c: &Content, ending: &str) {
        let p = &self.pet;
        if !p.alive() {
            return;
        }
        self.album.push(AlbumEntry {
            name: p.display_name(c),
            character: p.character.clone(),
            gen: p.gen,
            age: p.age_days(),
            job: p.job.clone(),
            partner: p.partner.clone(),
            ending: ending.into(),
        });
        if self.album.len() > 60 {
            let n = self.album.len() - 60;
            self.album.drain(0..n);
        }
    }

    pub fn new_egg(&mut self, c: &Content, keep_generation: bool, mystic: bool, legacy: Option<Skills>) {
        let gen = if keep_generation { self.pet.gen + 1 } else { 1 };
        self.pet = self.fresh_pet(c, gen, mystic, legacy.as_ref());
        self.emit(Event::NewEgg);
    }

    // ================================================================== player actions
    fn after_care(&mut self) {
        let p = &mut self.pet;
        if p.hunger > 0 && p.happy > 0 {
            p.zero_s = 0.0;
            p.need_called = false;
            if matches!(&p.call, Some(Call { reason: CallReason::Need, .. })) {
                p.call = None;
            }
        }
    }

    pub fn feed_meal(&mut self, c: &Content) -> Feed {
        if !self.pet.awake() {
            return Feed::Asleep;
        }
        if self.pet.hunger >= 4 {
            return Feed::Full;
        }
        if self.pet.stage != Stage::Baby && self.pet.discipline < 75 && self.rng.chance(c.rules.care.refuse_chance) {
            self.pet.scold_t = c.rules.care.refuse_window;
            return Feed::Refuse;
        }
        self.pet.hunger += 1;
        self.pet.weight += 1;
        self.after_care();
        Feed::Ok
    }

    pub fn feed_snack(&mut self) -> Feed {
        if !self.pet.awake() {
            return Feed::Asleep;
        }
        if self.pet.happy >= 4 && self.pet.hunger >= 4 {
            return Feed::Full;
        }
        self.pet.happy = (self.pet.happy + 1).min(4);
        self.pet.weight += 2;
        self.after_care();
        Feed::Ok
    }

    pub fn clean(&mut self) -> u8 {
        let n = self.pet.poop;
        self.pet.poop = 0;
        self.pet.poop_s = 0.0;
        n
    }

    pub fn medicine(&mut self) -> Medicine {
        let p = &mut self.pet;
        if !p.sick {
            return Medicine::NotSick;
        }
        p.doses = p.doses.saturating_sub(1);
        if p.doses == 0 {
            p.sick = false;
            p.sick_s = 0.0;
            Medicine::Cured
        } else {
            Medicine::Again
        }
    }

    pub fn toggle_lights(&mut self) -> bool {
        let p = &mut self.pet;
        p.lights = !p.lights;
        if !p.lights && matches!(&p.call, Some(Call { reason: CallReason::Lights, .. })) {
            p.call = None;
        }
        p.lights
    }

    pub fn scold(&mut self) -> Discipline {
        let p = &mut self.pet;
        if matches!(&p.call, Some(Call { reason: CallReason::Fake, .. })) {
            p.call = None;
            p.discipline = (p.discipline + 25).min(100);
            return Discipline::Good;
        }
        if p.scold_t > 0.0 {
            p.scold_t = 0.0;
            p.discipline = (p.discipline + 25).min(100);
            return Discipline::Good;
        }
        p.happy = p.happy.saturating_sub(1);
        Discipline::Mean
    }

    pub fn praise(&mut self) -> Discipline {
        if self.pet.praise_t > 0.0 {
            let p = &mut self.pet;
            p.praise_t = 0.0;
            p.discipline = (p.discipline + 25).min(100);
            p.happy = (p.happy + 1).min(4);
            self.after_care();
            return Discipline::Good;
        }
        let p = &mut self.pet;
        if matches!(&p.call, Some(Call { reason: CallReason::Fake, .. })) {
            p.call = None;
            p.discipline = p.discipline.saturating_sub(10);
            return Discipline::Spoiled;
        }
        Discipline::Nothing
    }

    /// Outcome of a solo mini-game.
    pub fn play_result(&mut self, c: &Content, skill: Option<Skill>, score: u32, win: bool, gp: i64) -> Gains {
        let mut g = Gains::default();
        if !self.pet.alive() {
            return g;
        }
        let base = c.stage_rule(self.pet.stage).weight;
        self.pet.weight = (self.pet.weight - 1).max(base);
        if win {
            self.pet.happy = (self.pet.happy + 1).min(4);
            self.after_care();
        }
        if let Some(s) = skill {
            let amt = if win { (2 + score / 2).min(15) } else { (score / 4).max(1) };
            self.pet.skills.add(s, amt);
            g.skill = Some((s, amt));
        }
        if gp > 0 {
            self.gp += gp;
            g.gp = gp;
        }
        g
    }

    pub fn school_result(&mut self, skill: Skill, correct: u32, total: u32) -> u32 {
        if !self.pet.alive() {
            return 0;
        }
        let amt = correct * 4 + if correct == total { 6 } else { 0 };
        self.pet.skills.add(skill, amt);
        if correct * 2 >= total {
            self.pet.happy = (self.pet.happy + 1).min(4);
            self.after_care();
        }
        amt
    }

    pub fn can_work(&self, c: &Content) -> bool {
        self.pet.job.is_some() && self.pet.t - self.pet.last_work >= c.rules.care.work_cooldown
    }

    pub fn work_result(&mut self, c: &Content, score: u32, max_score: u32) -> i64 {
        let Some(job) = self.pet.job.as_ref().and_then(|j| c.job(j)).cloned() else { return 0 };
        if !self.pet.alive() {
            return 0;
        }
        let frac = (score as f64 / max_score.max(1) as f64).clamp(0.0, 1.0);
        let pay = (job.pay as f64 * (0.4 + 0.8 * frac)) as i64;
        self.pet.last_work = self.pet.t;
        self.gp += pay;
        self.pet.skills.add(job.skill, 3);
        let base = c.stage_rule(self.pet.stage).weight;
        self.pet.weight = (self.pet.weight - 1).max(base);
        pay
    }

    pub fn buy(&mut self, c: &Content, id: &str) -> bool {
        let Some(item) = c.item(id) else { return false };
        if self.gp < item.price {
            return false;
        }
        self.gp -= item.price;
        *self.items.entry(id.to_string()).or_insert(0) += 1;
        true
    }

    /// Use an item. Returns result lines, or None if it can't be used right now.
    pub fn use_item(&mut self, c: &Content, id: &str) -> Option<Vec<String>> {
        let item = c.item(id)?.clone();
        if self.items.get(id).copied().unwrap_or(0) == 0 || !self.pet.awake() {
            return None;
        }
        let fx = &item.effects;
        if fx.cure > 0 && !self.pet.sick {
            return Some(vec!["NOT SICK".into()]);
        }
        let left = self.items.get_mut(id).unwrap();
        *left -= 1;
        if *left == 0 {
            self.items.remove(id);
        }
        let mut out = Vec::new();
        let p = &mut self.pet;
        if fx.happy > 0 {
            p.happy = (p.happy + fx.happy).min(4);
            out.push(format!("HAPPY +{}", fx.happy));
        }
        if fx.hunger > 0 {
            p.hunger = (p.hunger + fx.hunger).min(4);
            out.push(format!("FULL +{}", fx.hunger));
        }
        p.weight += fx.weight;
        if fx.cure > 0 {
            p.sick = false;
            p.sick_s = 0.0;
            p.doses = 0;
            out.push("CURED!".into());
        }
        for (s, v) in [(Skill::Humor, fx.humor), (Skill::Gorgeous, fx.gorgeous), (Skill::Passion, fx.passion)] {
            if v > 0 {
                p.skills.add(s, v);
                out.push(format!("{}+{}", &s.label()[..5], v));
            }
        }
        self.after_care();
        Some(out)
    }

    // ================================================================== mail, school, jobs
    fn deliver_mail(&mut self, c: &Content) {
        let care = &c.rules.care;
        let m = &c.rules.mail;
        if self.pet.mail.len() >= care.mail_max_waiting {
            return;
        }
        let st = self.pet.stage;
        let mut options: Vec<(MailKind, u32)> = vec![
            (MailKind::Gp, m.gp),
            (MailKind::Heart, m.heart),
            (MailKind::Trick, m.trick),
            (MailKind::Item, m.item),
        ];
        if matches!(st, Stage::Child | Stage::Teen) && self.pet.last_school_day != self.pet.age_days() as i64 {
            options.push((MailKind::School, m.school));
        }
        if st == Stage::Adult
            && self.pet.job.is_none()
            && !self.pet.mail.iter().any(|x| x.kind == MailKind::Job)
            && self.best_job(c).is_some()
        {
            options.push((MailKind::Job, m.job));
        }
        if matches!(st, Stage::Adult | Stage::Senior) && !self.pet.married {
            options.push((MailKind::Love, m.love));
        }
        let weights: Vec<u32> = options.iter().map(|o| o.1).collect();
        let kind = options[self.rng.weighted(&weights)].0;
        let mut mail = Mail { kind, t: self.pet.t, gp: None, item: None, job: None };
        match kind {
            MailKind::Gp => mail.gp = Some(*self.rng.pick(&m.gp_amounts).unwrap_or(&100)),
            MailKind::Item => mail.item = self.rng.pick(&m.items).cloned(),
            MailKind::School => self.pet.last_school_day = self.pet.age_days() as i64,
            MailKind::Job => mail.job = self.best_job(c),
            _ => {}
        }
        self.pet.mail.push(mail);
        self.emit(Event::Mail);
    }

    /// The best job the pet is close to qualifying for (70% of the requirement).
    pub fn best_job(&self, c: &Content) -> Option<String> {
        c.rules
            .job
            .iter()
            .filter(|j| self.pet.skills.get(j.skill) as f64 >= j.need as f64 * 0.7)
            .max_by_key(|j| j.need)
            .map(|j| j.id.clone())
    }

    /// Open the oldest letter, applying automatic effects. The bool is true for an expired invite.
    pub fn open_mail(&mut self, c: &Content) -> Option<(Mail, bool)> {
        if self.pet.mail.is_empty() {
            return None;
        }
        let m = self.pet.mail.remove(0);
        let mut expired = false;
        match m.kind {
            MailKind::Gp => self.gp += m.gp.unwrap_or(0),
            MailKind::Heart => {
                self.pet.happy = (self.pet.happy + 1).min(4);
                self.pet.hunger = (self.pet.hunger + 1).min(4);
                self.after_care();
            }
            MailKind::Trick => self.pet.happy = self.pet.happy.saturating_sub(1),
            MailKind::Item => {
                if let Some(i) = &m.item {
                    if c.item(i).is_some() {
                        *self.items.entry(i.clone()).or_insert(0) += 1;
                    }
                }
            }
            MailKind::School => {
                expired = self.pet.t - m.t > c.rules.care.school_expires
                    || !matches!(self.pet.stage, Stage::Child | Stage::Teen);
            }
            _ => {}
        }
        Some((m, expired))
    }

    pub fn interview(&mut self, c: &Content, job: &str, correct: u32) -> bool {
        let Some(j) = c.job(job) else { return false };
        if !matches!(self.pet.stage, Stage::Adult | Stage::Senior) {
            return false;
        }
        if self.pet.skills.get(j.skill) >= j.need && correct >= 2 {
            self.pet.job = Some(job.to_string());
            self.pet.last_work = -1e12;
            return true;
        }
        false
    }

    // ================================================================== marriage & friends
    pub fn can_marry(&self, c: &Content) -> bool {
        let p = &self.pet;
        if p.married {
            return false;
        }
        p.stage == Stage::Senior || (p.stage == Stage::Adult && p.age_days() >= c.rules.life.matchmaker_age_days)
    }

    pub fn matchmaker_candidates(&mut self, c: &Content, n: usize) -> Vec<(String, String)> {
        let senior = self.pet.stage == Stage::Senior;
        let pool: Vec<String> = if senior {
            vec![c.find(Stage::Senior, None, None).unwrap_or("senior").to_string()]
        } else {
            c.adults().iter().map(|a| a.id.clone()).filter(|id| *id != self.pet.character).collect()
        };
        (0..n)
            .filter_map(|_| self.rng.pick(&pool).cloned())
            .map(|id| {
                let name = c.name_of(&id);
                (id, name)
            })
            .collect()
    }

    /// The wedding: the parent is recorded and the next generation's egg arrives.
    /// Returns true when two seniors married (a rare egg).
    pub fn marry(&mut self, c: &Content, partner_name: &str, partner_senior: bool) -> bool {
        let mystic = self.pet.stage == Stage::Senior && partner_senior;
        self.pet.married = true;
        self.pet.partner = Some(partner_name.to_string());
        self.record_album(c, "MARRIED");
        let legacy = self.pet.skills.clone();
        self.new_egg(c, true, mystic, Some(legacy));
        mystic
    }

    pub fn add_friend(&mut self, id: &str, name: &str) -> u32 {
        let f = self.friends.entry(id.to_string()).or_default();
        f.name = name.to_string();
        f.count += 1;
        f.count
    }

    pub fn friend_count(&self, id: &str) -> u32 {
        self.friends.get(id).map(|f| f.count).unwrap_or(0)
    }

    // ================================================================== offline catch-up
    /// Simulate `seconds` of real time that passed while the app was closed (or the computer
    /// slept), starting at unix time `start`. `hour_at(ts)` gives the local hour for a timestamp.
    /// Returns the seconds simulated (capped at 14 days).
    pub fn catch_up(&mut self, c: &Content, seconds: f64, start: f64, hour_at: &dyn Fn(f64) -> f64) -> f64 {
        let seconds = seconds.clamp(0.0, 14.0 * DAY);
        let mut t = 0.0;
        while t < seconds && self.pet.stage != Stage::Gone {
            let step = (seconds - t).min(60.0);
            self.step(c, step, hour_at(start + t));
            t += step;
        }
        seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOON: f64 = 12.0;

    fn game() -> (Content, Game) {
        let c = Content::builtin();
        let g = Game::new(&c, Pcg32::new(7));
        (c, g)
    }

    fn run(g: &mut Game, c: &Content, secs: f64, hour: f64, step: f64) {
        let mut t = 0.0;
        while t < secs {
            g.step(c, step, hour);
            t += step;
        }
    }

    fn care(g: &mut Game, c: &Content) {
        while g.pet.hunger < 4 && matches!(g.feed_meal(c), Feed::Ok | Feed::Refuse) {}
        while g.pet.happy < 4 {
            g.feed_snack();
        }
        g.clean();
        while g.pet.sick && g.medicine() == Medicine::Again {}
    }

    #[test]
    fn sleep_window() {
        assert!(in_window(23.0, [22.0, 9.0]) && in_window(3.0, [22.0, 9.0]) && !in_window(12.0, [22.0, 9.0]));
    }

    #[test]
    fn hatch_and_good_baby_becomes_child_a() {
        let (c, mut g) = game();
        run(&mut g, &c, 61.0, NOON, 30.0);
        assert_eq!(g.pet.stage, Stage::Baby);
        for _ in 0..130 {
            care(&mut g, &c);
            g.step(&c, 30.0, NOON);
        }
        assert_eq!(g.pet.stage, Stage::Child);
        assert_eq!(g.pet.character, "child_a");
    }

    #[test]
    fn neglected_baby_becomes_child_b() {
        let (c, mut g) = game();
        run(&mut g, &c, 61.0, NOON, 30.0);
        run(&mut g, &c, 3700.0, NOON, 30.0);
        assert_eq!(g.pet.stage, Stage::Child);
        assert!(g.pet.baby_mistakes >= 2);
        assert_eq!(g.pet.character, "child_b");
    }

    #[test]
    fn unanswered_call_is_one_mistake() {
        let (c, mut g) = game();
        run(&mut g, &c, 61.0, NOON, 30.0);
        g.pet.hunger = 0;
        g.step(&c, 1.0, NOON);
        assert_eq!(g.pet.call.as_ref().unwrap().reason, CallReason::Need);
        let m = g.pet.mistakes;
        run(&mut g, &c, 960.0, NOON, 10.0);
        assert_eq!(g.pet.mistakes, m + 1);
    }

    #[test]
    fn great_care_reaches_secret_or_best() {
        let (c, mut g) = game();
        run(&mut g, &c, 61.0, NOON, 30.0);
        let mut t = 0.0;
        while g.pet.stage != Stage::Adult && t < 5.0 * DAY {
            care(&mut g, &c);
            g.pet.discipline = 100;
            if g.pet.stage != Stage::Baby {
                g.pet.skills.passion += 1;
            }
            g.step(&c, 600.0, NOON);
            t += 600.0;
        }
        assert_eq!(g.pet.stage, Stage::Adult);
        assert!(["s_p", "p_a"].contains(&g.pet.character.as_str()), "{}", g.pet.character);
    }

    #[test]
    fn lights_left_on_at_night_is_a_mistake() {
        let (c, mut g) = game();
        run(&mut g, &c, 61.0, NOON, 30.0);
        g.pet.stage = Stage::Child;
        g.pet.character = "child_a".into();
        care(&mut g, &c);
        g.step(&c, 1.0, 20.5);
        assert!(g.pet.asleep);
        let m = g.pet.mistakes;
        run(&mut g, &c, 960.0, 20.6, 30.0);
        assert_eq!(g.pet.mistakes, m + 1);
        g.step(&c, 1.0, 9.0);
        assert!(!g.pet.asleep && g.pet.lights);
    }

    #[test]
    fn speed_never_shifts_bedtime() {
        // 600x speed for a whole pet-day at noon: the pet must never fall asleep.
        let (c, mut g) = game();
        g.pet.stage = Stage::Child;
        g.pet.character = "child_a".into();
        for _ in 0..1440 {
            care(&mut g, &c);
            g.step(&c, 60.0, NOON);
            assert!(!g.pet.asleep);
        }
    }

    #[test]
    fn the_bedtime_shift_moves_the_schedule_and_speed_still_doesnt() {
        // a child sleeps 20:00-08:00; shifted 12 hours, it sleeps 08:00-20:00 instead
        let (c, mut g) = game();
        g.pet.stage = Stage::Child;
        g.pet.character = "child_a".into();
        g.settings.bedtime_shift = 12;
        care(&mut g, &c);
        g.step(&c, 1.0, 23.0);
        assert!(!g.pet.asleep, "awake through the player's night");
        g.step(&c, 1.0, 8.5);
        assert!(g.pet.asleep, "asleep through the player's day");
        g.step(&c, 1.0, 20.0);
        assert!(!g.pet.asleep);
        // at 600x, a whole pet-day at the player's midnight: never asleep
        for _ in 0..1440 {
            care(&mut g, &c);
            g.step(&c, 60.0, 0.0);
            assert!(!g.pet.asleep);
        }
        // the menu steps an hour at a time, round the whole day
        let mut s = Settings::default();
        let mut seen = Vec::new();
        for _ in 0..BEDTIME_SHIFTS.count() {
            s.next_bedtime_shift();
            seen.push(s.bedtime_shift);
        }
        assert_eq!(s.bedtime_shift, 0);
        assert_eq!((seen[0], seen[11], seen[12]), (1, 12, -11));
        assert_eq!(s.bedtime_label(), "+0H");
        assert_eq!(s.pet_hour(1.0), 1.0);
        s.bedtime_shift = -3;
        assert_eq!((s.bedtime_label(), s.pet_hour(22.0)), ("-3H".to_string(), 1.0));
    }

    #[test]
    fn themes_are_saved_numbered_and_deleted() {
        let mut s = Settings::default();
        assert!(!s.look_unsaved(), "a preset is never lost");
        s.look.step(crate::look::Field::Ring, 1);
        assert!(s.look_unsaved());
        assert_eq!(s.save_theme(), Some(0));
        assert_eq!(s.save_theme(), Some(0), "the same look isn't saved twice");
        assert_eq!((s.shell_label(), s.look_unsaved()), ("THEME 1".to_string(), false));
        s.look.step(crate::look::Field::Gloss, 1);
        assert_eq!(s.save_theme(), Some(1));
        let second = s.look;
        s.wear_theme(0);
        assert_eq!(s.shell_label(), "THEME 1");
        s.delete_theme(0);
        assert_eq!(s.themes, vec![second]);
        assert_eq!(s.shell_label(), "CUSTOM", "still worn, no longer saved");
        while s.themes.len() < MAX_THEMES {
            s.look.step(crate::look::Field::Body, 1);
            s.save_theme();
        }
        s.look.step(crate::look::Field::Body, 1);
        assert_eq!(s.save_theme(), None);
        assert_eq!(s.themes.len(), MAX_THEMES);
    }

    #[test]
    fn neglect_eventually_goes_home() {
        let (c, mut g) = game();
        run(&mut g, &c, 61.0, NOON, 30.0);
        run(&mut g, &c, 6.0 * DAY, NOON, 120.0);
        assert_eq!(g.pet.stage, Stage::Gone);
        assert_eq!(g.album.last().unwrap().ending, "WENT HOME");
    }

    #[test]
    fn catch_up_is_deterministic() {
        let (c, g0) = game();
        let mut a = g0.clone();
        let mut b = g0.clone();
        let hours = |ts: f64| (ts / 3600.0) % 24.0;
        a.catch_up(&c, 3.0 * DAY, 1_000_000.0, &hours);
        b.catch_up(&c, 3.0 * DAY, 1_000_000.0, &hours);
        assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    }

    #[test]
    fn marriage_starts_next_generation_with_legacy() {
        let (c, mut g) = game();
        g.pet.stage = Stage::Adult;
        g.pet.character = "p_a".into();
        g.pet.age_s = 6.0 * DAY;
        g.pet.skills.passion = 300;
        let old_id = g.pet.id;
        assert!(g.can_marry(&c));
        g.marry(&c, "HOTFOOT", false);
        assert_eq!(g.pet.stage, Stage::Egg);
        assert_eq!(g.pet.gen, 2);
        assert_eq!(g.pet.skills.passion, 30);
        assert_ne!(g.pet.id, old_id);
        assert_eq!(g.album.last().unwrap().ending, "MARRIED");
    }

    #[test]
    fn two_seniors_make_the_rare_adult() {
        let (c, mut g) = game();
        g.pet.stage = Stage::Senior;
        g.pet.character = "senior".into();
        assert!(g.marry(&c, "OLDBEAN", true));
        assert!(g.pet.mystic);
        assert_eq!(g.adult_for(&c), "s_m");
    }

    #[test]
    fn jobs() {
        let (c, mut g) = game();
        g.pet.stage = Stage::Adult;
        g.pet.character = "p_a".into();
        g.pet.skills.passion = 60;
        assert_eq!(g.best_job(&c).as_deref(), Some("CHEF"));
        assert!(g.interview(&c, "CHEF", 3));
        assert!(g.can_work(&c));
        assert!(g.work_result(&c, 5, 10) > 0);
        assert!(!g.can_work(&c));
    }

    #[test]
    fn results_for_a_departed_pet_change_nothing() {
        let (c, mut g) = game();
        g.pet.stage = Stage::Gone;
        let before = g.gp;
        assert_eq!(g.play_result(&c, Some(Skill::Humor), 20, true, 100), Gains::default());
        assert_eq!(g.gp, before);
    }

    #[test]
    fn mail_and_items() {
        let (c, mut g) = game();
        g.pet.stage = Stage::Child;
        g.pet.character = "child_a".into();
        care(&mut g, &c);
        for _ in 0..20 {
            g.deliver_mail(&c);
        }
        assert_eq!(g.pet.mail.len(), c.rules.care.mail_max_waiting);
        while g.open_mail(&c).is_some() {}
        g.gp = 1000;
        assert!(g.buy(&c, "JOKES"));
        let h = g.pet.skills.humor;
        assert!(g.use_item(&c, "JOKES").is_some());
        assert_eq!(g.pet.skills.humor, h + 8);
    }
}
