//! Care screens: short animations, status, clock, and the food / discipline / game menus.

use crate::content::{Pose, Skill, Stage};
use crate::games::{self, GameHost, SoloGame};
use crate::lcd::Lcd;
use crate::rng::Pcg32;
use crate::sim::{Discipline, Feed};
use crate::sprite::Sprite;
use crate::{Button, Tune};

use super::{back, boxed, draw_hearts, msg, msg_owned, Ctx, DrawCx, ListView, Pick, Res, Screen, Tr};

// ====================================================================== reactions
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReactKind {
    No,
    Happy,
    Sad,
    Angry,
}

/// A short reaction from the pet (about 1.6 s). Pops with `Res::Done`.
pub struct React {
    kind: ReactKind,
    t: f64,
    started: bool,
}

impl React {
    pub fn new(kind: ReactKind) -> React {
        React { kind, t: 0.0, started: false }
    }
}

impl Screen for React {
    fn name(&self) -> &'static str {
        "React"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if !self.started {
            self.started = true;
            cx.play(match self.kind {
                ReactKind::No | ReactKind::Sad => Tune::No,
                ReactKind::Happy => Tune::Ok,
                ReactKind::Angry => Tune::Hit,
            });
        }
        self.t += dt;
        if self.t >= 1.6 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn press(&mut self, _cx: &mut Ctx, b: Button) -> Tr {
        if b != Button::A && self.t > 0.3 {
            self.t = 1.6;
        }
        Tr::None
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let f = (self.t * 4.0) as i32 % 2 == 1;
        let ch = &cx.game.pet.character;
        let c = cx.content;
        match self.kind {
            ReactKind::No => lcd.blit(c.sprite(ch, Pose::Sad), 8, 0, f, false, None),
            ReactKind::Happy => {
                lcd.draw(c.sprite(ch, Pose::Happy), 8, if f { -1 } else { 0 });
                if f {
                    lcd.draw(c.prop("star"), 1, 2);
                    lcd.draw(c.prop("star"), 26, 6);
                }
            }
            ReactKind::Angry => {
                lcd.draw(c.sprite(ch, Pose::Sad), 8 + if f { 1 } else { -1 }, 0);
                lcd.text("!", 26, 1);
            }
            ReactKind::Sad => {
                lcd.draw(c.sprite(ch, Pose::Sad), 8, 0);
                if f {
                    lcd.vline(5, 6, 7);
                }
            }
        }
    }
}

/// Eating animation: the food shrinks in three bites.
pub struct Eat {
    food: Sprite,
    t: f64,
}

impl Eat {
    pub fn new(food: Sprite) -> Eat {
        Eat { food, t: 0.0 }
    }
}

impl Screen for Eat {
    fn name(&self) -> &'static str {
        "Eat"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        let before = (self.t * 2.5) as i32;
        self.t += dt;
        if (self.t * 2.5) as i32 != before {
            cx.play(Tune::Tick);
        }
        if self.t >= 2.6 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn press(&mut self, _cx: &mut Ctx, b: Button) -> Tr {
        if b != Button::A {
            self.t = 2.6;
        }
        Tr::None
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let bites = ((self.t / 0.75) as i32).min(3);
        if bites < 3 {
            lcd.blit(&self.food, 2, 7, false, false, Some(self.food.w - bites * 3));
        }
        let pose = if (self.t * 3.0) as i32 % 2 == 0 { Pose::Eat } else { Pose::Idle1 };
        lcd.blit(cx.content.sprite(&cx.game.pet.character, pose), 13, 0, true, false, None);
    }
}

/// Flushing the poop away.
pub struct Flush {
    poops: u8,
    t: f64,
    started: bool,
}

impl Flush {
    pub fn new(poops: u8) -> Flush {
        Flush { poops, t: 0.0, started: false }
    }
}

pub const POOP_POS: [(i32, i32); 4] = [(26, 10), (26, 4), (20, 10), (20, 4)];

impl Screen for Flush {
    fn name(&self) -> &'static str {
        "Flush"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if !self.started {
            self.started = true;
            cx.play(Tune::Ok);
        }
        self.t += dt;
        if self.t >= 1.6 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let c = cx.content;
        let edge = (32.0 - self.t * 26.0) as i32;
        lcd.draw(c.sprite(&cx.game.pet.character, Pose::Idle1), 0, 0);
        for &(x, y) in POOP_POS.iter().take(self.poops as usize) {
            lcd.draw(c.prop("poop1"), x, y);
        }
        for y in 0..16 {
            for x in edge.max(0)..32 {
                lcd.set(x, y, 0);
            }
            lcd.dot(edge + if y % 4 < 2 { 1 } else { 0 }, y);
        }
    }
}

/// Giving medicine.
pub struct MedicineAnim {
    t: f64,
}

impl MedicineAnim {
    pub fn new() -> MedicineAnim {
        MedicineAnim { t: 0.0 }
    }
}

impl Default for MedicineAnim {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for MedicineAnim {
    fn name(&self) -> &'static str {
        "Medicine"
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        if self.t >= 1.6 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let pose = if self.t < 1.0 { Pose::Sad } else { Pose::Idle1 };
        lcd.draw(cx.content.sprite(&cx.game.pet.character, pose), 0, 0);
        let x = (30.0 - self.t.min(1.0) * 14.0) as i32;
        lcd.blit(cx.content.prop("syringe"), x, 6, true, false, None);
    }
}

/// Evolution: static, then the new form with its name.
pub struct Evolve {
    character: String,
    t: f64,
    started: bool,
}

impl Evolve {
    pub fn new(character: String) -> Evolve {
        Evolve { character, t: 0.0, started: false }
    }
}

impl Screen for Evolve {
    fn name(&self) -> &'static str {
        "Evolve"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if !self.started {
            self.started = true;
            cx.play(Tune::Evolve);
        }
        self.t += dt;
        if self.t > 7.0 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn press(&mut self, _cx: &mut Ctx, b: Button) -> Tr {
        if self.t > 2.2 && b != Button::A {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        if self.t < 2.0 {
            let mut r = Pcg32::new((self.t * 8.0) as u64 + 1);
            for y in 0..16 {
                for x in 0..32 {
                    if r.chance(0.5) {
                        lcd.dot(x, y);
                    }
                }
            }
            return;
        }
        let pose = if (self.t * 2.0) as i32 % 2 == 1 { Pose::Happy } else { Pose::Idle1 };
        lcd.draw(cx.content.sprite(&self.character, pose), 0, 0);
        let name = cx.content.name_of(&self.character);
        let (a, b) = name.split_at(name.len().min(4));
        lcd.text(a, 17, 2);
        lcd.text(&b.chars().take(4).collect::<String>(), 17, 9);
    }
}

pub struct Hatch {
    t: f64,
    started: bool,
}

impl Hatch {
    pub fn new() -> Hatch {
        Hatch { t: 0.0, started: false }
    }
}

impl Default for Hatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for Hatch {
    fn name(&self) -> &'static str {
        "Hatch"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if !self.started {
            self.started = true;
            cx.play(Tune::Hatch);
        }
        self.t += dt;
        if self.t > 3.0 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn press(&mut self, _cx: &mut Ctx, _b: Button) -> Tr {
        if self.t > 1.5 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        if self.t < 1.5 {
            lcd.draw(cx.content.prop("egg_crack"), 8 + ((self.t * 8.0) as i32 % 2), 0);
        } else {
            lcd.draw(cx.content.sprite(&cx.game.pet.character, Pose::Happy), 0, 0);
            lcd.text("HI!", 18, 5);
        }
    }
}

/// Two pets and a heart. Pops with `Res::Done`.
pub struct Wedding {
    mine: String,
    theirs: String,
    t: f64,
    started: bool,
}

impl Wedding {
    pub fn new(mine: String, theirs: String) -> Wedding {
        Wedding { mine, theirs, t: 0.0, started: false }
    }
}

impl Screen for Wedding {
    fn name(&self) -> &'static str {
        "Wedding"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if !self.started {
            self.started = true;
            cx.play(Tune::Wed);
        }
        self.t += dt;
        if self.t > 6.0 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn press(&mut self, _cx: &mut Ctx, _b: Button) -> Tr {
        if self.t > 2.5 {
            Tr::PopWith(Res::Done)
        } else {
            Tr::None
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let f = (self.t * 2.0) as i32 % 2;
        let c = cx.content;
        lcd.draw(c.sprite(&self.mine, Pose::Happy), -2, 0);
        lcd.blit(c.sprite(&self.theirs, Pose::Happy), 18, 0, true, false, None);
        lcd.draw(c.prop("heart"), 13, 2 - f);
    }
}

/// The pet leaves on a UFO; B starts a new egg.
pub struct Gone {
    character: String,
    t: f64,
    started: bool,
}

impl Gone {
    pub fn new(character: String) -> Gone {
        Gone { character, t: 0.0, started: false }
    }
}

impl Screen for Gone {
    fn name(&self) -> &'static str {
        "Gone"
    }
    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        if !self.started {
            self.started = true;
            cx.play(Tune::Gone);
        }
        self.t += dt;
        Tr::None
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        if self.t >= 3.0 && b == Button::B {
            if cx.game.pet.stage == Stage::Gone {
                let c = cx.content;
                cx.game.new_egg(c, false, false, None);
            }
            cx.play(Tune::Ok);
            return Tr::ToMain;
        }
        Tr::None
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let c = cx.content;
        if self.t < 3.0 {
            let y = (-self.t * 5.0) as i32;
            lcd.draw(c.prop("ufo"), 11, y + 1);
            if self.t < 1.2 {
                lcd.draw(c.sprite(&self.character, Pose::Sad), 8, y + 6);
            }
            return;
        }
        lcd.line("WENT HOME TO ITS PLANET", 1, self.t - 3.0, false);
        lcd.text(if (self.t as i32) % 2 == 1 { "B:NEWEGG" } else { "B: EGG" }, 0, 9);
    }
}

// ====================================================================== clock & status
pub struct Clock {
    t: f64,
}

impl Clock {
    pub fn new() -> Clock {
        Clock { t: 0.0 }
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for Clock {
    fn name(&self) -> &'static str {
        "Clock"
    }
    fn leavable(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        Tr::None
    }
    fn press(&mut self, _cx: &mut Ctx, _b: Button) -> Tr {
        Tr::Pop
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        use chrono::{Datelike, Local, Timelike};
        let now = Local::now();
        let colon = if (self.t * 2.0) as i32 % 2 == 0 { ":" } else { " " };
        lcd.text_center(&format!("{:02}{}{:02}", now.hour(), colon, now.minute()), 2);
        let days = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
        lcd.text_center(&format!("{} {}", days[now.weekday().num_days_from_monday() as usize], now.day()), 9);
    }
}

#[derive(Clone, Copy)]
enum Page {
    Name,
    Age,
    Hunger,
    Happy,
    Training,
    Skill(Skill),
    Points,
    Job,
}

pub struct Status {
    i: usize,
    t: f64,
}

impl Status {
    pub fn new() -> Status {
        Status { i: 0, t: 0.0 }
    }
    fn pages(stage: Stage) -> Vec<Page> {
        if stage == Stage::Egg {
            return vec![Page::Name, Page::Points];
        }
        let mut v = vec![Page::Name, Page::Age, Page::Hunger, Page::Happy, Page::Training];
        v.extend(Skill::ALL.iter().map(|&s| Page::Skill(s)));
        v.extend([Page::Points, Page::Job]);
        v
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for Status {
    fn name(&self) -> &'static str {
        "Status"
    }
    fn leavable(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        Tr::None
    }
    fn prev(&mut self, cx: &mut Ctx) -> Option<Tr> {
        cx.play(Tune::Click);
        self.i = back(self.i, Status::pages(cx.game.pet.stage).len());
        self.t = 0.0;
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        let n = Status::pages(cx.game.pet.stage).len();
        match b {
            Button::A | Button::B => {
                cx.play(Tune::Click);
                self.i += 1;
                self.t = 0.0;
                if self.i >= n {
                    if b == Button::B {
                        return Tr::Pop;
                    }
                    self.i = 0;
                }
                Tr::None
            }
            Button::C => Tr::Pop,
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let p = &cx.game.pet;
        let c = cx.content;
        let pages = Status::pages(p.stage);
        match pages[self.i % pages.len()] {
            Page::Name => {
                lcd.line(&p.display_name(c), 1, self.t, false);
                lcd.line(&c.name_of(&p.character), 9, self.t, false);
            }
            Page::Age => {
                lcd.text(&format!("AGE {}", p.age_days()), 0, 1);
                lcd.text(&format!("{}G G{}", p.weight, p.gen), 0, 9);
            }
            Page::Hunger => {
                lcd.text("HUNGRY", 0, 1);
                draw_hearts(lcd, c, p.hunger, 9);
            }
            Page::Happy => {
                lcd.text("HAPPY", 0, 1);
                draw_hearts(lcd, c, p.happy, 9);
            }
            Page::Training => {
                lcd.text("TRAINING", 0, 1);
                lcd.rect(0, 9, 32, 6, false);
                lcd.rect(1, 10, (30 * p.discipline as i32) / 100, 4, true);
            }
            Page::Skill(s) => {
                lcd.line(s.label(), 1, self.t, false);
                let v = p.skills.get(s);
                lcd.text(&v.to_string(), 0, 9);
                lcd.rect(18, 10, (v / 20).min(13) as i32, 4, true);
            }
            Page::Points => {
                lcd.text("POINTS", 0, 1);
                lcd.text(&format!("{}GP", cx.game.gp), 0, 9);
            }
            Page::Job => {
                lcd.text("JOB", 0, 1);
                lcd.line(p.job.as_deref().unwrap_or("NONE"), 9, self.t, false);
            }
        }
    }
}

// ====================================================================== food menu
pub struct FoodMenu {
    view: ListView,
    picking: Option<(u64, Vec<String>)>,
}

impl FoodMenu {
    pub fn new() -> FoodMenu {
        FoodMenu { view: ListView::default(), picking: None }
    }
}

impl Default for FoodMenu {
    fn default() -> Self {
        Self::new()
    }
}

/// Reaction followed by a message.
pub fn react_then(kind: ReactKind, pages: &[(&str, &str)]) -> Tr {
    Tr::PushAll(vec![msg(pages), boxed(React::new(kind))])
}

/// The screens shown after using an item.
pub fn item_used(cx: &Ctx, id: &str, lines: Vec<String>) -> Tr {
    let c = cx.content;
    let food = c.item(id).map(|i| i.food).unwrap_or(false);
    let sprite = if food { c.prop("snack").clone() } else { c.prop("gift").clone() };
    let mut pages: Vec<(String, String)> = lines.into_iter().map(|l| (id.to_string(), l)).collect();
    if pages.is_empty() {
        pages.push((id.to_string(), String::new()));
    }
    Tr::PushAll(vec![msg_owned(pages), boxed(Eat::new(sprite))])
}

impl Screen for FoodMenu {
    fn name(&self) -> &'static str {
        "FoodMenu"
    }
    fn leavable(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.view.t += dt;
        Tr::None
    }
    fn prev(&mut self, cx: &mut Ctx) -> Option<Tr> {
        cx.play(Tune::Click);
        self.view.prev(3);
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::A => {
                cx.play(Tune::Click);
                self.view.next(3);
                Tr::None
            }
            Button::C => {
                cx.play(Tune::Click);
                Tr::Pop
            }
            Button::B => {
                cx.play(Tune::Click);
                let c = cx.content;
                match self.view.i {
                    0 | 1 => {
                        let r = if self.view.i == 0 { cx.game.feed_meal(c) } else { cx.game.feed_snack() };
                        let food = if self.view.i == 0 { "meal" } else { "snack" };
                        match r {
                            Feed::Ok => Tr::Push(boxed(Eat::new(c.prop(food).clone()))),
                            Feed::Refuse => react_then(ReactKind::No, &[("NO!", "SCOLD ME?")]),
                            Feed::Full => react_then(ReactKind::No, &[("FULL!", "")]),
                            Feed::Asleep => Tr::Push(msg(&[("ZZZ...", "")])),
                        }
                    }
                    _ => {
                        let foods: Vec<String> = cx
                            .game
                            .items
                            .keys()
                            .filter(|k| c.item(k).map(|i| i.food).unwrap_or(false))
                            .cloned()
                            .collect();
                        let rows = foods.iter().map(|k| (k.clone(), format!("X{}", cx.game.items[k]))).collect();
                        self.picking = Some((cx.game.pet.id, foods));
                        Tr::Push(boxed(Pick::new(rows).empty("NO FOOD", "SHOP!")))
                    }
                }
            }
        }
    }
    fn result(&mut self, cx: &mut Ctx, r: Res) -> Tr {
        if let (Some((pet, ids)), Res::Pick(i)) = (self.picking.take(), r) {
            if !cx.same_pet(pet) {
                return Tr::None;
            }
            let id = ids[i].clone();
            let c = cx.content;
            return match cx.game.use_item(c, &id) {
                Some(lines) => item_used(cx, &id, lines),
                None => Tr::Push(msg(&[("CAN'T", "NOW")])),
            };
        }
        Tr::None
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        self.view.draw(&["MEAL".into(), "SNACK".into(), "ITEMS".into()], lcd);
    }
}

// ====================================================================== discipline
pub struct DisciplineMenu {
    view: ListView,
}

impl DisciplineMenu {
    pub fn new() -> DisciplineMenu {
        DisciplineMenu { view: ListView::default() }
    }
}

impl Default for DisciplineMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for DisciplineMenu {
    fn name(&self) -> &'static str {
        "DisciplineMenu"
    }
    fn leavable(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.view.t += dt;
        Tr::None
    }
    fn prev(&mut self, cx: &mut Ctx) -> Option<Tr> {
        cx.play(Tune::Click);
        self.view.prev(2);
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::A => {
                cx.play(Tune::Click);
                self.view.next(2);
                Tr::None
            }
            Button::C => Tr::Pop,
            Button::B => {
                if self.view.i == 0 {
                    match cx.game.scold() {
                        Discipline::Good => react_then(ReactKind::Sad, &[("GOOD", "TRAINING")]),
                        _ => react_then(ReactKind::Angry, &[("WHY?", "HAPPY -1")]),
                    }
                } else {
                    match cx.game.praise() {
                        Discipline::Good => react_then(ReactKind::Happy, &[("GOOD", "TRAINING")]),
                        Discipline::Spoiled => react_then(ReactKind::Happy, &[("SPOILED!", "")]),
                        _ => Tr::Push(boxed(React::new(ReactKind::Happy))),
                    }
                }
            }
        }
    }
    fn draw(&self, _cx: &DrawCx, lcd: &mut Lcd) {
        self.view.draw(&["SCOLD".into(), "PRAISE".into()], lcd);
    }
}

// ====================================================================== game menu
enum GamePending {
    None,
    Solo { pet: u64, game: SoloGame },
    Work { pet: u64, max: u32 },
}

pub struct GameMenu {
    view: ListView,
    pending: GamePending,
}

impl GameMenu {
    pub fn new() -> GameMenu {
        GameMenu { view: ListView::default(), pending: GamePending::None }
    }
    fn labels(cx_has_job: bool) -> Vec<String> {
        let mut v: Vec<String> = SoloGame::ALL.iter().map(|g| g.label().to_string()).collect();
        if cx_has_job {
            v.push("WORK".into());
        }
        v
    }
}

impl Default for GameMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for GameMenu {
    fn name(&self) -> &'static str {
        "GameMenu"
    }
    fn leavable(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.view.t += dt;
        Tr::None
    }
    fn prev(&mut self, cx: &mut Ctx) -> Option<Tr> {
        cx.play(Tune::Click);
        self.view.prev(GameMenu::labels(cx.game.pet.job.is_some()).len());
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        let labels = GameMenu::labels(cx.game.pet.job.is_some());
        match b {
            Button::A => {
                cx.play(Tune::Click);
                self.view.next(labels.len());
                Tr::None
            }
            Button::C => Tr::Pop,
            Button::B => {
                cx.play(Tune::Click);
                let i = self.view.i % labels.len();
                let seed = cx.game.rng.next_u64();
                if i < SoloGame::ALL.len() {
                    let g = SoloGame::ALL[i];
                    self.pending = GamePending::Solo { pet: cx.game.pet.id, game: g };
                    return Tr::Push(boxed(GameHost::new(g.make(), seed)));
                }
                // WORK
                let c = cx.content;
                if !cx.game.can_work(c) {
                    return Tr::Push(msg(&[("DAY OFF", "REST")]));
                }
                let job = c.job(cx.game.pet.job.as_deref().unwrap_or("")).cloned();
                let Some(job) = job else { return Tr::None };
                let game = games::job_game(job.game);
                let max = game.spec().max_score;
                let (title, howto) = (game.spec().title, game.spec().howto);
                self.pending = GamePending::Work { pet: cx.game.pet.id, max };
                Tr::PushAll(vec![
                    boxed(GameHost::new(game, seed)),
                    msg_owned(vec![("WORK".into(), job.id.clone()), (title.into(), howto.into())]),
                ])
            }
        }
    }
    fn result(&mut self, cx: &mut Ctx, r: Res) -> Tr {
        let pending = std::mem::replace(&mut self.pending, GamePending::None);
        let Res::Score(score) = r else {
            // the work briefing finished: keep waiting for the game itself
            if let (GamePending::Work { .. }, Res::Done) = (&pending, &r) {
                self.pending = pending;
            }
            return Tr::None;
        };
        let c = cx.content;
        match pending {
            GamePending::Solo { pet, game } if cx.same_pet(pet) => {
                let spec = game.make().spec();
                let win = score >= spec.win_score;
                let gp = spec.gp_per_point as i64 * score as i64;
                let gains = cx.game.play_result(c, game.skill(), score, win, gp);
                let mut pages = vec![(if win { "WIN!" } else { "TRY" }.to_string(), format!("SCORE {score}"))];
                if let Some((s, v)) = gains.skill {
                    pages.push((format!("+{v}"), s.label().to_string()));
                }
                if gains.gp > 0 {
                    pages.push((format!("+{}", gains.gp), "GP".into()));
                }
                let m = super::Message::new(pages).with_sound(if win { Tune::Win } else { Tune::Lose });
                Tr::PushAll(vec![boxed(m), boxed(React::new(if win { ReactKind::Happy } else { ReactKind::Sad }))])
            }
            GamePending::Work { pet, max } if cx.same_pet(pet) => {
                let pay = cx.game.work_result(c, score, max);
                let m = super::Message::new(vec![
                    ("PAYDAY!".into(), format!("+{pay}GP")),
                    ("SCORE".into(), format!("{score}/{max}")),
                ])
                .with_sound(Tune::Win);
                Tr::Push(boxed(m))
            }
            _ => Tr::None,
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        self.view.draw(&GameMenu::labels(cx.game.pet.job.is_some()), lcd);
    }
}
