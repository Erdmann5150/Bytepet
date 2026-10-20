//! Town: mail, shop, items, matchmaker, friends, album and setup.
//!
//! Multi-step flows (school, job interview, matchmaker, starting over) are explicit states in
//! `Flow`. Each records the id of the pet that started it; if that pet is gone when a result
//! comes back, the result is dropped.

use crate::content::{Skill, Stage};
use crate::games::{GameHost, Quiz};
use crate::lcd::Lcd;
use crate::sim::MailKind;
use crate::{Button, Tune};

use super::care::{item_used, react_then, ReactKind, Wedding};
use super::{
    back, boxed, draw_pick, msg, msg_owned, Confirm, Ctx, DrawCx, ListView, Message, Pick, Request, Res, Screen, Tr,
};

enum Flow {
    None,
    SchoolConfirm { pet: u64 },
    SchoolTeacher { pet: u64 },
    SchoolQuiz { pet: u64, skill: Skill },
    JobBrief { pet: u64, job: String },
    JobConfirm { pet: u64, job: String },
    JobQuiz { pet: u64, job: String },
    PartnerPick { pet: u64, cands: Vec<(String, String)> },
    PartnerConfirm { pet: u64, cand: (String, String) },
    Wedding { pet: u64, cand: (String, String) },
    Album { entries: Vec<(String, String)> },
}

pub struct TownMenu {
    view: ListView,
    flow: Flow,
}

const ITEMS: [&str; 7] = ["MAIL", "SHOP", "ITEMS", "PARTNER", "FRIENDS", "ALBUM", "SETUP"];

impl TownMenu {
    pub fn new() -> TownMenu {
        TownMenu { view: ListView::default(), flow: Flow::None }
    }

    fn labels(cx: &DrawCx) -> Vec<String> {
        let n = cx.game.pet.mail.len();
        ITEMS.iter().map(|&s| if s == "MAIL" && n > 0 { format!("MAIL {n}") } else { s.to_string() }).collect()
    }

    fn read_mail(&mut self, cx: &mut Ctx) -> Tr {
        if !cx.game.pet.alive() {
            return Tr::Push(msg(&[("NO MAIL", "")]));
        }
        let c = cx.content;
        let Some((m, expired)) = cx.game.open_mail(c) else {
            return Tr::Push(msg(&[("NO MAIL", "")]));
        };
        cx.play(Tune::Mail);
        let pet = cx.game.pet.id;
        match m.kind {
            MailKind::Gp => Tr::Push(boxed(
                Message::new(vec![("A GIFT!".into(), format!("+{}GP", m.gp.unwrap_or(0)))]).with_sound(Tune::Win),
            )),
            MailKind::Heart => Tr::Push(msg(&[("A CARD!", "HEARTS +1")])),
            MailKind::Trick => react_then(ReactKind::Sad, &[("SNAKE IN", "A BOX!"), ("HAPPY -1", "")]),
            MailKind::Item => Tr::Push(msg_owned(vec![("GOT A".into(), m.item.clone().unwrap_or_default())])),
            MailKind::Love => Tr::Push(msg(&[("LOVE", "LETTER!"), ("VISIT", "PARTNER")])),
            MailKind::School => {
                if expired {
                    Tr::Push(msg(&[("SCHOOL", "EXPIRED")]))
                } else {
                    self.flow = Flow::SchoolConfirm { pet };
                    Tr::Push(boxed(Confirm::new("SCHOOL INVITE! GO?")))
                }
            }
            MailKind::Job => {
                let job = m.job.clone().unwrap_or_default();
                let Some(j) = c.job(&job) else { return Tr::None };
                self.flow = Flow::JobBrief { pet, job: job.clone() };
                Tr::Push(msg_owned(vec![
                    ("JOB".into(), "OFFER!".into()),
                    (job, format!("{} {}", &j.skill.label()[..5], j.need)),
                ]))
            }
        }
    }

    fn partner(&mut self, cx: &mut Ctx) -> Tr {
        let c = cx.content;
        if cx.game.pet.married {
            return Tr::Push(msg(&[("MARRIED", "")]));
        }
        if !cx.game.can_marry(c) {
            let age = c.rules.life.matchmaker_age_days;
            return Tr::Push(msg_owned(vec![("TOO".into(), "YOUNG".into()), ("ADULTS".into(), format!("AGE {age}+"))]));
        }
        let cands = cx.game.matchmaker_candidates(c, 3);
        let rows = cands.iter().map(|(_, n)| (n.clone(), "B:MEET".to_string())).collect();
        self.flow = Flow::PartnerPick { pet: cx.game.pet.id, cands };
        Tr::PushAll(vec![boxed(Pick::new(rows)), msg(&[("MATCH-", "MAKER!")])])
    }
}

impl Default for TownMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for TownMenu {
    fn name(&self) -> &'static str {
        "TownMenu"
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
        self.view.prev(ITEMS.len());
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::A => {
                cx.play(Tune::Click);
                self.view.next(ITEMS.len());
                Tr::None
            }
            Button::C => {
                cx.play(Tune::Click);
                Tr::Pop
            }
            Button::B => {
                cx.play(Tune::Click);
                self.flow = Flow::None;
                match ITEMS[self.view.i % ITEMS.len()] {
                    "MAIL" => self.read_mail(cx),
                    "SHOP" => Tr::Push(boxed(Shop::default())),
                    "ITEMS" => Tr::Push(boxed(Items::default())),
                    "PARTNER" => self.partner(cx),
                    "FRIENDS" => {
                        let c = cx.content;
                        let mut f: Vec<_> = cx.game.friends.values().cloned().collect();
                        f.sort_by_key(|b| std::cmp::Reverse(b.count));
                        let rows = f.iter().map(|f| (f.name.clone(), c.friend_level(f.count).to_string())).collect();
                        Tr::Push(boxed(Pick::new(rows).empty("NO", "FRIENDS")))
                    }
                    "ALBUM" => {
                        let c = cx.content;
                        let entries: Vec<(String, String)> = cx
                            .game
                            .album
                            .iter()
                            .rev()
                            .map(|e| {
                                let mut detail = format!("{}|{}", c.name_of(&e.character), e.ending);
                                if let Some(j) = &e.job {
                                    detail.push_str(&format!("|JOB|{j}"));
                                }
                                if let Some(p) = &e.partner {
                                    detail.push_str(&format!("|PARTNER|{p}"));
                                }
                                (format!("{}|G{} AGE{}", e.name, e.gen, e.age), detail)
                            })
                            .collect();
                        let rows = entries
                            .iter()
                            .map(|(a, _)| {
                                let mut it = a.split('|');
                                (it.next().unwrap_or("").to_string(), it.next().unwrap_or("").to_string())
                            })
                            .collect();
                        self.flow = Flow::Album { entries };
                        Tr::Push(boxed(Pick::new(rows).empty("ALBUM", "EMPTY")))
                    }
                    _ => Tr::Push(boxed(Setup::default())),
                }
            }
        }
    }

    fn result(&mut self, cx: &mut Ctx, r: Res) -> Tr {
        let flow = std::mem::replace(&mut self.flow, Flow::None);
        let c = cx.content;
        match (flow, r) {
            (Flow::SchoolConfirm { pet }, Res::Yes(true)) if cx.same_pet(pet) => {
                self.flow = Flow::SchoolTeacher { pet };
                let rows = Skill::ALL.iter().map(|s| (s.label().to_string(), "TEACHER".to_string())).collect();
                Tr::Push(boxed(Pick::new(rows)))
            }
            (Flow::SchoolTeacher { pet }, Res::Pick(i)) if cx.same_pet(pet) => {
                let skill = Skill::ALL[i.min(2)];
                self.flow = Flow::SchoolQuiz { pet, skill };
                let seed = cx.game.rng.next_u64();
                Tr::Push(boxed(GameHost::new(Box::new(Quiz::new("CLASS", 5)), seed)))
            }
            (Flow::SchoolQuiz { pet, skill }, Res::Score(n)) if cx.same_pet(pet) => {
                let amt = cx.game.school_result(skill, n, 5);
                let m = Message::new(vec![
                    (format!("{n}/5 RIGHT"), String::new()),
                    (format!("+{amt}"), skill.label().into()),
                ])
                .with_sound(if n >= 3 { Tune::Win } else { Tune::Lose });
                Tr::Push(boxed(m))
            }
            (Flow::JobBrief { pet, job }, Res::Done) if cx.same_pet(pet) => {
                self.flow = Flow::JobConfirm { pet, job };
                Tr::Push(boxed(Confirm::new("INTERVIEW?")))
            }
            (Flow::JobConfirm { pet, job }, Res::Yes(true)) if cx.same_pet(pet) => {
                self.flow = Flow::JobQuiz { pet, job };
                let seed = cx.game.rng.next_u64();
                Tr::Push(boxed(GameHost::new(Box::new(Quiz::new("INTERVW", 3)), seed)))
            }
            (Flow::JobQuiz { pet, job }, Res::Score(n)) if cx.same_pet(pet) => {
                if cx.game.interview(c, &job, n) {
                    Tr::Push(boxed(
                        Message::new(vec![
                            ("HIRED!".into(), String::new()),
                            ("NEW JOB".into(), job),
                            ("PLAY".into(), "GAME>WORK".into()),
                        ])
                        .with_sound(Tune::Win),
                    ))
                } else {
                    let need = c.job(&job).map(|j| format!("{} {}", &j.skill.label()[..5], j.need)).unwrap_or_default();
                    Tr::Push(boxed(
                        Message::new(vec![("NOT".into(), "HIRED".into()), ("NEED".into(), need)])
                            .with_sound(Tune::Lose),
                    ))
                }
            }
            (Flow::PartnerPick { pet, cands }, Res::Pick(i)) if cx.same_pet(pet) => {
                let cand = cands[i.min(cands.len() - 1)].clone();
                self.flow = Flow::PartnerConfirm { pet, cand: cand.clone() };
                Tr::Push(boxed(Confirm::new(format!("MARRY {}?", cand.1))))
            }
            (Flow::PartnerPick { pet, cands }, Res::Done) => {
                // the matchmaker's greeting closed; keep waiting for the pick
                self.flow = Flow::PartnerPick { pet, cands };
                Tr::None
            }
            (Flow::PartnerConfirm { pet, cand }, Res::Yes(true)) if cx.same_pet(pet) => {
                self.flow = Flow::Wedding { pet, cand: cand.clone() };
                Tr::Push(boxed(Wedding::new(cx.game.pet.character.clone(), cand.0)))
            }
            (Flow::Wedding { pet, cand }, Res::Done) if cx.same_pet(pet) => wed(cx, &cand.1, cand.0 == "senior"),
            (Flow::Album { entries }, Res::Pick(i)) => {
                let detail = &entries[i.min(entries.len() - 1)].1;
                let parts: Vec<&str> = detail.split('|').collect();
                let pages = parts.chunks(2).map(|p| (p[0].to_string(), p.get(1).unwrap_or(&"").to_string())).collect();
                self.flow = Flow::Album { entries };
                Tr::Push(msg_owned(pages))
            }
            (Flow::Album { entries }, Res::Done) => {
                self.flow = Flow::Album { entries };
                Tr::None
            }
            _ => Tr::None,
        }
    }

    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        self.view.draw(&TownMenu::labels(cx), lcd);
    }
}

/// Finish a wedding (from the matchmaker or a link): record it, start the next egg.
pub fn wed(cx: &mut Ctx, partner_name: &str, partner_senior: bool) -> Tr {
    let c = cx.content;
    let mystic = cx.game.marry(c, partner_name, partner_senior);
    let mut pages = vec![("AN EGG!".to_string(), String::new())];
    if mystic {
        pages.push(("A RARE".into(), "EGG!".into()));
    }
    pages.push((format!("GEN {}", cx.game.pet.gen), String::new()));
    Tr::ToMainThen(vec![boxed(Message::new(pages).with_sound(Tune::Hatch))])
}

// ====================================================================== shop & items
#[derive(Default)]
pub struct Shop {
    i: usize,
    t: f64,
}

impl Screen for Shop {
    fn name(&self) -> &'static str {
        "Shop"
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
        self.i = back(self.i, cx.content.rules.item.len());
        self.t = 0.0;
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        let c = cx.content;
        let n = c.rules.item.len();
        match b {
            Button::A => {
                cx.play(Tune::Click);
                self.i = (self.i + 1) % n.max(1);
                self.t = 0.0;
                Tr::None
            }
            Button::C => Tr::Pop,
            Button::B => {
                let item = c.rules.item[self.i % n].clone();
                if cx.game.buy(c, &item.id) {
                    Tr::Push(boxed(
                        Message::new(vec![("BOUGHT".into(), item.id), ("LEFT".into(), format!("{}GP", cx.game.gp))])
                            .with_sound(Tune::Ok),
                    ))
                } else {
                    Tr::Push(boxed(
                        Message::new(vec![
                            ("NEED".into(), format!("{}GP", item.price)),
                            ("HAVE".into(), format!("{}GP", cx.game.gp)),
                        ])
                        .with_sound(Tune::No),
                    ))
                }
            }
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let items = &cx.content.rules.item;
        if let Some(it) = items.get(self.i % items.len().max(1)) {
            draw_pick(lcd, &it.id, &format!("{}GP", it.price), self.t, self.i, items.len());
        }
    }
}

#[derive(Default)]
pub struct Items {
    i: usize,
    t: f64,
}

impl Screen for Items {
    fn name(&self) -> &'static str {
        "Items"
    }
    fn leavable(&self) -> bool {
        true
    }
    fn update(&mut self, _cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        Tr::None
    }
    fn prev(&mut self, cx: &mut Ctx) -> Option<Tr> {
        let n = cx.game.items.len();
        if n == 0 {
            return None;
        }
        cx.play(Tune::Click);
        self.i = back(self.i, n);
        self.t = 0.0;
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        let ids: Vec<String> = cx.game.items.keys().cloned().collect();
        match b {
            Button::A if !ids.is_empty() => {
                cx.play(Tune::Click);
                self.i = (self.i + 1) % ids.len();
                self.t = 0.0;
                Tr::None
            }
            Button::B if !ids.is_empty() => {
                let id = ids[self.i % ids.len()].clone();
                let c = cx.content;
                match cx.game.use_item(c, &id) {
                    Some(lines) => item_used(cx, &id, lines),
                    None => Tr::Push(msg(&[("CAN'T", "NOW")])),
                }
            }
            Button::C => Tr::Pop,
            _ => Tr::None,
        }
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let ids: Vec<(&String, &u32)> = cx.game.items.iter().collect();
        if ids.is_empty() {
            draw_pick(lcd, "NO ITEMS", "", self.t, 0, 0);
            return;
        }
        let i = self.i % ids.len();
        draw_pick(lcd, ids[i].0, &format!("X{}", ids[i].1), self.t, i, ids.len());
    }
}

// ====================================================================== setup (on the device)
#[derive(Default)]
pub struct Setup {
    i: usize,
    t: f64,
    confirming: bool,
}

/// WHENOFF: whether time keeps running while Bytepet is closed (caught up on the next start).
const SETUP_ROWS: [&str; 6] = ["SPEED", "SOUND", "SHELL", "WHENOFF", "MORE", "NEWEGG"];
pub const SPEEDS: [u32; 4] = [1, 10, 60, 600];
pub const SHELL_COUNT: u32 = 8;

impl Screen for Setup {
    fn name(&self) -> &'static str {
        "Setup"
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
        self.i = back(self.i, SETUP_ROWS.len());
        self.t = 0.0;
        Some(Tr::None)
    }
    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match b {
            Button::A => {
                cx.play(Tune::Click);
                self.i = (self.i + 1) % SETUP_ROWS.len();
                self.t = 0.0;
                Tr::None
            }
            Button::C => Tr::Pop,
            Button::B => {
                cx.play(Tune::Ok);
                self.t = 0.0;
                let s = &mut cx.game.settings;
                match SETUP_ROWS[self.i] {
                    "SPEED" => {
                        let k = SPEEDS.iter().position(|&x| x == s.speed).unwrap_or(0);
                        s.speed = SPEEDS[(k + 1) % SPEEDS.len()];
                    }
                    "SOUND" => s.set_sound(s.next_sound()),
                    "SHELL" => s.shell = (s.shell + 1) % SHELL_COUNT,
                    "WHENOFF" => s.run_away = !s.run_away,
                    "MORE" => cx.ui.request = Some(Request::Settings),
                    _ => {
                        self.confirming = true;
                        return Tr::Push(boxed(Confirm::new("START OVER? PET WILL LEAVE")));
                    }
                }
                Tr::None
            }
        }
    }
    fn result(&mut self, cx: &mut Ctx, r: Res) -> Tr {
        if std::mem::take(&mut self.confirming) && r == Res::Yes(true) {
            let c = cx.content;
            cx.game.record_album(c, "RESET");
            cx.game.new_egg(c, false, false, None);
            return Tr::ToMain;
        }
        Tr::None
    }
    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let s = &cx.game.settings;
        let (a, b) = match SETUP_ROWS[self.i] {
            "SPEED" => ("SPEED".to_string(), format!("{}X", s.speed)),
            "SOUND" => ("SOUND".to_string(), s.sound_label()),
            "SHELL" => ("SHELL".to_string(), crate::layout::SHELLS[(s.shell % SHELL_COUNT) as usize].name.to_string()),
            "WHENOFF" => ("WHEN OFF".to_string(), if s.run_away { "RUNS" } else { "PAUSE" }.to_string()),
            "MORE" => ("MORE".to_string(), "B:MENU".to_string()),
            _ => ("NEW EGG".to_string(), "B:RESET".to_string()),
        };
        draw_pick(lcd, &a, &b, self.t, self.i, SETUP_ROWS.len());
    }
}

/// Used by the link screen and tests.
pub fn stage_can_link(stage: Stage) -> bool {
    !matches!(stage, Stage::Egg | Stage::Baby | Stage::Gone)
}
