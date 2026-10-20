//! The LINK screen: pairs two Bytepets over the LAN.
//!
//! Flow: one side HOSTs, the other JOINs (found automatically, or typed IP). On connecting, the
//! joiner shows a 4-digit code and sends it; the host shows the same code and must press B to
//! accept, so nobody else on the network can link to you unasked. Both sides check the
//! protocol version. The host then chooses VISIT, GAME, GIFT or MARRY.

use std::time::Instant;

use serde_json::{json, Value};

use crate::content::{Pose, Stage};
use crate::games::{GameHost, VersusGame};
use crate::lcd::Lcd;
use crate::net::{clip, local_ip, Found, Link, NetEvent, PROTO};
use crate::screens::care::Wedding;
use crate::screens::town::wed;
use crate::screens::{boxed, msg_owned, Confirm, Ctx, DrawCx, Key, Message, Pick, Res, Screen, Tr};
use crate::{Button, Tune};

const ACTIONS: [&str; 4] = ["VISIT", "GAME", "GIFT", "MARRY"];

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub character: String,
    pub can_marry: bool,
    pub senior: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum State {
    Menu,
    Hosting,
    Scanning,
    PickHost,
    IpEntry,
    Connecting,
    /// Host: guest's code shown, waiting for B.
    Approve,
    /// Guest: showing our code, waiting for the host to accept.
    AwaitAccept,
    Action,
    Wait,
    Visit,
    AwaitScore,
    AwaitAnswer,
    Proposal,
    GiftPick,
    Wedding,
}

pub struct LinkScreen {
    link: Option<Link>,
    state: State,
    k: usize,
    t: f64,
    since: Instant,
    host: bool,
    code: String,
    peer: Option<Peer>,
    accepted: bool,
    hosts: Vec<Found>,
    ip_text: String,
    my_ip: String,
    my_score: Option<u32>,
    peer_score: Option<u32>,
    gift_ids: Vec<String>,
    pet_id: u64,
}

impl LinkScreen {
    /// A link screen frozen in one state, for documentation art (no network involved).
    /// `state` is "host", "code" or "visit".
    #[doc(hidden)]
    pub fn preview(state: &str, peer_character: &str) -> LinkScreen {
        let mut s = LinkScreen::new();
        s.my_ip = "192.168.1.20".into();
        s.code = "4821".into();
        s.host = true;
        s.peer = Some(Peer {
            id: "preview".into(),
            name: "BOB".into(),
            character: peer_character.into(),
            can_marry: false,
            senior: false,
        });
        s.state = match state {
            "host" => State::Hosting,
            "code" => State::Approve,
            _ => State::Visit,
        };
        s.t = 0.6;
        s
    }

    pub fn new() -> LinkScreen {
        LinkScreen {
            link: None,
            state: State::Menu,
            k: 0,
            t: 0.0,
            since: Instant::now(),
            host: false,
            code: String::new(),
            peer: None,
            accepted: false,
            hosts: Vec::new(),
            ip_text: String::new(),
            my_ip: local_ip(),
            my_score: None,
            peer_score: None,
            gift_ids: Vec::new(),
            pet_id: 0,
        }
    }

    fn set(&mut self, s: State) {
        self.state = s;
        self.t = 0.0;
        self.k = 0;
        self.since = Instant::now();
    }

    fn send(&self, v: Value) {
        if let Some(l) = &self.link {
            l.send(&v);
        }
    }

    /// Close the link and leave, optionally showing a message.
    fn finish(&mut self, pages: Vec<(String, String)>, tune: Option<Tune>) -> Tr {
        if let Some(mut l) = self.link.take() {
            if l.connected() {
                l.send(&json!({"t": "bye"}));
            }
            l.close();
        }
        if pages.is_empty() {
            Tr::PopWith(Res::Done)
        } else {
            let mut m = Message::new(pages);
            if let Some(t) = tune {
                m = m.with_sound(t);
            }
            Tr::Replace(boxed(m))
        }
    }

    fn fin(&mut self, a: &str, b: &str, tune: Option<Tune>) -> Tr {
        self.finish(vec![(a.into(), b.into())], tune)
    }

    fn hello(&self, cx: &Ctx) -> Value {
        let p = &cx.game.pet;
        let c = cx.content;
        let mut v = json!({
            "t": "hello", "proto": PROTO, "id": cx.game.device_id, "name": clip(&p.display_name(c), 8),
            "char": p.character, "stage": p.stage, "can_marry": cx.game.can_marry(c),
            "senior": p.stage == Stage::Senior,
        });
        if !self.host {
            v["code"] = json!(self.code);
        }
        v
    }

    fn befriend(&self, cx: &mut Ctx) -> String {
        match &self.peer {
            Some(p) => {
                let n = cx.game.add_friend(&p.id, &p.name);
                cx.content.friend_level(n).to_string()
            }
            None => String::new(),
        }
    }

    fn soulmates(&self, cx: &Ctx) -> bool {
        self.peer.as_ref().map(|p| cx.game.friend_count(&p.id) >= cx.content.soulmate_links()).unwrap_or(false)
    }

    fn start_game(&mut self, kind: VersusGame, seed: u64) -> Tr {
        self.my_score = None;
        self.peer_score = None;
        self.set(State::AwaitScore);
        Tr::Push(boxed(GameHost::new(kind.make(), seed)))
    }

    fn host_action(&mut self, cx: &mut Ctx, act: &str) -> Tr {
        let c = cx.content;
        match act {
            "VISIT" => {
                self.send(json!({"t": "visit"}));
                self.set(State::Visit);
                Tr::None
            }
            "GAME" => {
                let kind = VersusGame::ALL[cx.game.rng.below(3)];
                let seed = cx.game.rng.next_u64() >> 11; // fits in a JSON number exactly
                self.send(json!({"t": "game", "kind": kind.id(), "seed": seed}));
                self.start_game(kind, seed)
            }
            "GIFT" => {
                self.gift_ids = cx.game.items.keys().cloned().collect();
                if cx.game.gp >= 100 {
                    self.gift_ids.push("100GP".into());
                }
                let rows = self.gift_ids.iter().map(|i| (i.clone(), "B:SEND".to_string())).collect();
                self.set(State::GiftPick);
                Tr::Push(boxed(Pick::new(rows).empty("NOTHING", "TO GIVE")))
            }
            _ => {
                let peer = self.peer.clone().unwrap();
                let why = if !cx.game.can_marry(c) || !peer.can_marry {
                    Some(("NOT", "ELIGIBLE"))
                } else if cx.game.pet.stage == Stage::Senior && !peer.senior {
                    Some(("SENIORS", "ONLY"))
                } else if !self.soulmates(cx) {
                    Some(("NEED", "SOULMATE"))
                } else {
                    None
                };
                if let Some((a, b)) = why {
                    cx.play(Tune::No);
                    return Tr::Push(msg_owned(vec![(a.into(), b.into())]));
                }
                self.send(json!({"t": "propose"}));
                self.set(State::AwaitAnswer);
                Tr::None
            }
        }
    }

    fn handle(&mut self, cx: &mut Ctx, e: NetEvent) -> Tr {
        match e {
            NetEvent::Error(m) => self.fin("LINK", &m, Some(Tune::No)),
            NetEvent::ScanDone => {
                if self.state == State::Scanning {
                    self.hosts = self.link.as_ref().map(|l| l.found.lock().unwrap().clone()).unwrap_or_default();
                    if self.hosts.is_empty() {
                        return self.finish(
                            vec![("NOBODY".into(), "FOUND".into()), ("TRY".into(), "IP ENTRY".into())],
                            Some(Tune::No),
                        );
                    }
                    self.set(State::PickHost);
                }
                Tr::None
            }
            NetEvent::Connected => {
                self.host = self.state == State::Hosting;
                if !self.host {
                    self.code = format!("{:04}", cx.game.rng.below(10000));
                    self.set(State::AwaitAccept);
                }
                let h = self.hello(cx);
                self.send(h);
                Tr::None
            }
            NetEvent::Closed => {
                // the peer may finish first and hang up once it has both scores
                let result_known = self.state == State::AwaitScore && self.peer_score.is_some();
                if self.state == State::Visit || self.state == State::Wedding || result_known {
                    Tr::None
                } else {
                    self.fin("LINK", "ENDED", Some(Tune::No))
                }
            }
            NetEvent::Msg(m) => self.handle_msg(cx, m),
        }
    }

    fn handle_msg(&mut self, cx: &mut Ctx, m: Value) -> Tr {
        let c = cx.content;
        let t = m["t"].as_str().unwrap_or("");
        match t {
            "hello" => {
                if m["proto"].as_u64() != Some(PROTO) {
                    self.send(json!({"t": "reject", "why": "VERSION"}));
                    return self.fin("OTHER", "VERSION", Some(Tune::No));
                }
                let ch = m["char"].as_str().unwrap_or("baby");
                self.peer = Some(Peer {
                    id: m["id"].as_str().unwrap_or("?").chars().take(24).collect(),
                    name: clip(m["name"].as_str().unwrap_or("?"), 8),
                    character: if c.character(ch).is_some() { ch.to_string() } else { "baby".into() },
                    can_marry: m["can_marry"].as_bool().unwrap_or(false),
                    senior: m["senior"].as_bool().unwrap_or(false),
                });
                if self.host {
                    self.code =
                        m["code"].as_str().unwrap_or("").chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                    cx.play(Tune::Ok);
                    self.set(State::Approve);
                } else if self.accepted {
                    self.set(State::Wait);
                }
                Tr::None
            }
            "accept" if !self.host => {
                self.accepted = true;
                cx.play(Tune::Ok);
                if self.peer.is_some() {
                    self.set(State::Wait);
                }
                Tr::None
            }
            "reject" => self.fin("LINK", "REFUSED", Some(Tune::No)),
            "bye" => {
                if matches!(self.state, State::Visit | State::Wedding | State::AwaitScore) {
                    Tr::None
                } else {
                    self.fin("LINK", "ENDED", None)
                }
            }
            "visit" if !self.host && self.state == State::Wait => {
                self.set(State::Visit);
                Tr::None
            }
            "game" if !self.host && self.state == State::Wait => {
                match VersusGame::from_id(m["kind"].as_str().unwrap_or("")) {
                    Some(kind) => self.start_game(kind, m["seed"].as_u64().unwrap_or(1)),
                    None => self.fin("UNKNOWN", "GAME", Some(Tune::No)),
                }
            }
            "score" if self.state == State::AwaitScore => {
                self.peer_score = Some(m["v"].as_u64().unwrap_or(0).min(1000) as u32);
                Tr::None
            }
            "gift" if !self.host && self.state == State::Wait => {
                let what = if let Some(gp) = m["gp"].as_i64() {
                    let gp = gp.clamp(0, 1000);
                    cx.game.gp += gp;
                    format!("{gp}GP")
                } else {
                    let item = m["item"].as_str().unwrap_or("");
                    if c.item(item).is_none() {
                        return self.fin("ODD", "GIFT", None);
                    }
                    *cx.game.items.entry(item.to_string()).or_insert(0) += 1;
                    item.to_string()
                };
                let lvl = self.befriend(cx);
                self.finish(vec![("GOT".into(), what), ("FRIEND".into(), lvl)], Some(Tune::Win))
            }
            "propose" if !self.host && self.state == State::Wait => {
                let senior_ok =
                    cx.game.pet.stage != Stage::Senior || self.peer.as_ref().map(|p| p.senior).unwrap_or(false);
                if !(cx.game.can_marry(c) && self.soulmates(cx) && senior_ok) {
                    self.send(json!({"t": "answer", "yes": false}));
                    return self.finish(
                        vec![("PROPOSAL".into(), String::new()), ("NOT".into(), "ELIGIBLE".into())],
                        Some(Tune::No),
                    );
                }
                self.set(State::Proposal);
                let name = self.peer.as_ref().map(|p| p.name.clone()).unwrap_or_default();
                Tr::Push(boxed(Confirm::new(format!("{name} PROPOSES! MARRY?"))))
            }
            "answer" if self.state == State::AwaitAnswer => {
                if m["yes"].as_bool().unwrap_or(false) {
                    self.start_wedding(cx)
                } else {
                    self.fin("SAID NO", "", Some(Tune::Lose))
                }
            }
            _ => Tr::None,
        }
    }

    fn start_wedding(&mut self, cx: &mut Ctx) -> Tr {
        self.pet_id = cx.game.pet.id;
        self.befriend(cx);
        self.set(State::Wedding);
        let theirs = self.peer.as_ref().map(|p| p.character.clone()).unwrap_or_default();
        Tr::Push(boxed(Wedding::new(cx.game.pet.character.clone(), theirs)))
    }
}

impl Default for LinkScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for LinkScreen {
    fn name(&self) -> &'static str {
        "Link"
    }
    /// Only the first menu; never mid-connection.
    fn leavable(&self) -> bool {
        self.state == State::Menu
    }

    fn wants_text(&self) -> bool {
        self.state == State::IpEntry
    }

    fn press(&mut self, cx: &mut Ctx, b: Button) -> Tr {
        match (&self.state, b) {
            (State::Menu, Button::A) => {
                cx.play(Tune::Click);
                self.k = (self.k + 1) % 3;
                Tr::None
            }
            (State::Menu, Button::B) => {
                cx.play(Tune::Click);
                let mut link = Link::new();
                match self.k {
                    0 => {
                        let name = cx.game.pet.display_name(cx.content);
                        if link.host(&name).is_err() {
                            return self.fin("LINK", "FAILED", Some(Tune::No));
                        }
                        self.link = Some(link);
                        self.set(State::Hosting);
                    }
                    1 => {
                        link.discover(3.0);
                        self.link = Some(link);
                        self.set(State::Scanning);
                    }
                    _ => {
                        self.link = Some(link);
                        self.ip_text.clear();
                        self.set(State::IpEntry);
                    }
                }
                Tr::None
            }
            (State::Menu, Button::C) => self.finish(vec![], None),
            (State::PickHost, Button::A) if !self.hosts.is_empty() => {
                cx.play(Tune::Click);
                self.k = (self.k + 1) % self.hosts.len();
                Tr::None
            }
            (State::PickHost, Button::B) if !self.hosts.is_empty() => {
                let h = self.hosts[self.k].clone();
                if let Some(l) = &self.link {
                    l.join(&h.ip, h.port);
                }
                self.set(State::Connecting);
                Tr::None
            }
            (State::IpEntry, Button::B) if !self.ip_text.is_empty() => self.key(cx, Key::Enter),
            (State::Approve, Button::B) => {
                cx.play(Tune::Ok);
                self.send(json!({"t": "accept"}));
                self.set(State::Action);
                Tr::None
            }
            (State::Approve, Button::C) => {
                self.send(json!({"t": "reject"}));
                self.fin("REFUSED", "", None)
            }
            (State::Action, Button::A) => {
                cx.play(Tune::Click);
                self.k = (self.k + 1) % ACTIONS.len();
                Tr::None
            }
            (State::Action, Button::B) => {
                cx.play(Tune::Click);
                let act = ACTIONS[self.k];
                self.host_action(cx, act)
            }
            (State::Action, Button::C) => self.fin("BYE BYE", "", None),
            (State::Visit, _) => {
                if self.t > 1.5 {
                    self.t = 99.0;
                }
                Tr::None
            }
            (_, Button::C) => self.fin("CANCELED", "", None),
            _ => Tr::None,
        }
    }

    fn text(&mut self, _cx: &mut Ctx, s: &str) -> Tr {
        if self.state == State::IpEntry {
            for ch in s.chars() {
                if (ch.is_ascii_digit() || ch == '.' || ch == ':') && self.ip_text.len() < 21 {
                    self.ip_text.push(ch);
                }
            }
        }
        Tr::None
    }

    fn key(&mut self, _cx: &mut Ctx, k: Key) -> Tr {
        if self.state != State::IpEntry {
            return Tr::None;
        }
        match k {
            Key::Backspace => {
                self.ip_text.pop();
            }
            Key::Enter if !self.ip_text.is_empty() => {
                // "192.168.1.5" or "192.168.1.5:41976"
                let (ip, port) = match self.ip_text.split_once(':') {
                    Some((ip, p)) => (ip.to_string(), p.parse().unwrap_or(crate::net::TCP_PORT)),
                    None => (self.ip_text.clone(), crate::net::TCP_PORT),
                };
                if let Some(l) = &self.link {
                    l.join(&ip, port);
                }
                self.set(State::Connecting);
            }
            _ => {}
        }
        Tr::None
    }

    fn update(&mut self, cx: &mut Ctx, dt: f64) -> Tr {
        self.t += dt;
        let events = self.link.as_ref().map(|l| l.poll()).unwrap_or_default();
        for e in events {
            let tr = self.handle(cx, e);
            if !matches!(tr, Tr::None) {
                return tr;
            }
        }
        let waited = self.since.elapsed().as_secs_f64();
        match self.state {
            State::Visit if self.t > 4.0 => {
                let lvl = self.befriend(cx);
                self.finish(vec![("FRIEND".into(), lvl)], Some(Tune::Ok))
            }
            State::AwaitScore => {
                if let (Some(a), Some(b)) = (self.my_score, self.peer_score) {
                    self.befriend(cx);
                    if a > b {
                        cx.game.pet.happy = (cx.game.pet.happy + 1).min(4);
                        self.fin("YOU WIN!", &format!("{a}-{b}"), Some(Tune::Win))
                    } else if a < b {
                        self.fin("YOU LOSE", &format!("{a}-{b}"), Some(Tune::Lose))
                    } else {
                        self.fin("DRAW", &format!("{a}-{b}"), Some(Tune::Ok))
                    }
                } else if self.my_score.is_some() && waited > 60.0 {
                    self.fin("NO SCORE", "FROM PEER", Some(Tune::No))
                } else {
                    Tr::None
                }
            }
            State::Scanning if waited > 8.0 => self.fin("NOBODY", "FOUND", Some(Tune::No)),
            State::Hosting if waited > 300.0 => self.fin("TIMED", "OUT", Some(Tune::No)),
            State::Connecting | State::AwaitAccept | State::Approve if waited > 60.0 => {
                self.fin("TIMED", "OUT", Some(Tune::No))
            }
            _ => Tr::None,
        }
    }

    fn result(&mut self, cx: &mut Ctx, r: Res) -> Tr {
        match self.state {
            State::AwaitScore => {
                // our versus game finished (or was backed out of, which counts as zero)
                if self.my_score.is_none() {
                    let n = if let Res::Score(n) = r { n } else { 0 };
                    self.my_score = Some(n);
                    self.since = Instant::now();
                    self.send(json!({"t": "score", "v": n}));
                }
                Tr::None
            }
            State::GiftPick => match r {
                Res::Pick(i) if i < self.gift_ids.len() => {
                    let id = self.gift_ids[i].clone();
                    if id == "100GP" {
                        cx.game.gp -= 100;
                        self.send(json!({"t": "gift", "gp": 100}));
                    } else {
                        if let Some(n) = cx.game.items.get_mut(&id) {
                            *n -= 1;
                            if *n == 0 {
                                cx.game.items.remove(&id);
                            }
                        }
                        self.send(json!({"t": "gift", "item": id}));
                    }
                    let lvl = self.befriend(cx);
                    self.finish(vec![("SENT".into(), id), ("FRIEND".into(), lvl)], Some(Tune::Ok))
                }
                _ => {
                    self.set(State::Action);
                    Tr::None
                }
            },
            State::Proposal => match r {
                Res::Yes(true) => {
                    self.send(json!({"t": "answer", "yes": true}));
                    self.start_wedding(cx)
                }
                Res::Yes(false) => {
                    self.send(json!({"t": "answer", "yes": false}));
                    self.fin("SAID NO", "", None)
                }
                _ => Tr::None,
            },
            State::Wedding if r == Res::Done => {
                let peer = self.peer.clone();
                if let Some(mut l) = self.link.take() {
                    l.send(&json!({"t": "bye"}));
                    l.close();
                }
                match peer {
                    Some(p) if cx.same_pet(self.pet_id) => wed(cx, &p.name, p.senior),
                    _ => Tr::ToMain,
                }
            }
            _ => Tr::None,
        }
    }

    fn draw(&self, cx: &DrawCx, lcd: &mut Lcd) {
        let c = cx.content;
        match &self.state {
            State::Menu => {
                let opts = ["HOST", "JOIN", "IP"];
                lcd.text(&format!(">{}", opts[self.k]), 0, 1);
                lcd.text(&format!(" {}", opts[(self.k + 1) % 3]), 0, 9);
            }
            State::Hosting => {
                lcd.text(if (self.t * 2.0) as i32 % 2 == 0 { "WAITING" } else { "WAITING." }, 0, 1);
                let port = self.link.as_ref().and_then(|l| l.port).unwrap_or(crate::net::TCP_PORT);
                let addr =
                    if port == crate::net::TCP_PORT { self.my_ip.clone() } else { format!("{}:{port}", self.my_ip) };
                lcd.line(&format!("IP {addr}"), 9, self.t, false);
            }
            State::Scanning | State::Connecting => {
                lcd.text(if self.state == State::Scanning { "SEARCH" } else { "LINKING" }, 0, 1);
                let r = (self.t * 4.0) as i32 % 4;
                for i in 0..r {
                    lcd.hline(12 - i * 3, 19 + i * 3, 14 - i * 2);
                }
            }
            State::PickHost => {
                if let Some(h) = self.hosts.get(self.k) {
                    lcd.line(&h.name, 1, self.t, false);
                    lcd.line(&h.ip, 9, self.t, false);
                }
            }
            State::IpEntry => {
                lcd.text("HOST IP?", 0, 1);
                let shown: String = self.ip_text.chars().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect();
                lcd.text(&shown, 0, 9);
                if (self.t * 3.0) as i32 % 2 == 1 {
                    let x = shown.len() as i32 * 4;
                    lcd.hline(x, x + 2, 14);
                }
            }
            State::Approve => {
                lcd.text(&format!("CODE{}", self.code), 0, 1);
                // two prompts that each fit, alternating
                lcd.text(if (self.t * 0.8) as i32 % 2 == 0 { "B:ACCEPT" } else { "C:REFUSE" }, 0, 9);
            }
            State::AwaitAccept => {
                lcd.text(&format!("CODE{}", self.code), 0, 1);
                lcd.line("CONFIRM ON THE HOST", 9, self.t, false);
            }
            State::Action => {
                lcd.text(&format!(">{}", ACTIONS[self.k]), 0, 1);
                if let Some(p) = &self.peer {
                    lcd.line(&format!("W/{}", p.name), 9, self.t, false);
                }
            }
            State::Wait => {
                lcd.text("LINKED!", 0, 1);
                if let Some(p) = &self.peer {
                    lcd.line(&p.name, 9, self.t, false);
                }
            }
            State::Visit => {
                let f = (self.t * 2.0) as i32 % 2;
                lcd.draw(c.sprite(&cx.game.pet.character, Pose::Happy), -2, -f);
                let theirs = self.peer.as_ref().map(|p| p.character.as_str()).unwrap_or("baby");
                lcd.blit(c.sprite(theirs, Pose::Happy), 18, f - 1, true, false, None);
                if f == 1 {
                    lcd.draw(c.prop("heart_small"), 15, 2);
                }
            }
            State::AwaitAnswer => {
                lcd.text("ASKING", 0, 1);
                lcd.draw(c.prop("ring"), 13, 9);
            }
            State::AwaitScore => {
                lcd.text("WAITING", 0, 1);
                lcd.text("SCORES", 0, 9);
            }
            State::Proposal | State::GiftPick | State::Wedding => {}
        }
    }
}
