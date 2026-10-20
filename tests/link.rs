//! Two Bytepets linking over loopback: pairing, visit, versus game, gift, refused and accepted
//! proposals. Uses real sockets.

mod common;

use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use bytepet::content::Stage;
use bytepet::Button;
use common::Rig;

/// The link tests share the default port, so they must not run at the same time.
static SERIAL: Mutex<()> = Mutex::new(());

fn both(a: &mut Rig, b: &mut Rig, n: usize) {
    for _ in 0..n {
        a.frames(1);
        b.frames(1);
        sleep(Duration::from_millis(3));
    }
}

fn wait(a: &mut Rig, b: &mut Rig, cond: impl Fn(&Rig, &Rig) -> bool, max: usize) -> bool {
    for _ in 0..max {
        if cond(a, b) {
            return true;
        }
        both(a, b, 1);
    }
    cond(a, b)
}

/// Host on `a`, join `b` by IP, approve the pairing code. Leaves `a` on the action menu.
fn connect(a: &mut Rig, b: &mut Rig) {
    a.core.click_icon(7);
    b.core.click_icon(7);
    a.tap(Button::B); // HOST
    b.taps("AA"); // IP
    b.tap(Button::B);
    b.core.text("127.0.0.1");
    b.core.key(bytepet::screens::Key::Enter);
    let lcd_says = |r: &Rig, text: &str| r.core.top_name() == "Link" && lcd_text(r).contains(text);
    assert!(wait(a, b, |a, b| lcd_says(a, "CODE") && lcd_says(b, "CODE"), 600), "no pairing code shown");
    a.tap(Button::B); // approve
    assert!(wait(a, b, |_, b| lcd_says(b, "LINKED"), 600), "guest never linked");
}

/// A crude LCD reader: compares the screen against rendered text lines.
fn lcd_text(r: &Rig) -> String {
    let mut out = String::new();
    for word in ["CODE", "LINKED", "VISIT", "WAITING"] {
        let mut probe = bytepet::lcd::Lcd::default();
        probe.text(word, 0, 1);
        let matches = (0..32).all(|x| (1..6).all(|y| !probe.get(x, y) || r.core.lcd.get(x, y)));
        if matches {
            out.push_str(word);
        }
    }
    out
}

#[allow(dead_code)]
fn dump(r: &Rig) {
    for y in 0..16 {
        let row: String = (0..32).map(|x| if r.core.lcd.get(x, y) { '#' } else { '.' }).collect();
        eprintln!("{row}");
    }
    eprintln!();
}

fn adult(seed: u64, ch: &str) -> Rig {
    let mut r = Rig::with_pet(seed, Stage::Adult, ch);
    r.core.game.pet.age_s = 6.0 * 86400.0;
    r
}

#[test]
fn link_visit_game_gift_and_marriage() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut a = adult(11, "p_a");
    let mut b = adult(12, "g_b");

    // VISIT
    connect(&mut a, &mut b);
    a.tap(Button::B);
    assert!(wait(&mut a, &mut b, |a, b| a.top() == "Message" && b.top() == "Message", 900));
    let (ida, idb) = (a.core.game.device_id.clone(), b.core.game.device_id.clone());
    assert_eq!(a.core.game.friend_count(&idb), 1);
    assert_eq!(b.core.game.friend_count(&ida), 1);
    a.back_to_main();
    b.back_to_main();

    // GAME: both play the same seeded round
    connect(&mut a, &mut b);
    a.taps("A");
    a.tap(Button::B);
    assert!(wait(&mut a, &mut b, |a, b| a.top() == "Game" && b.top() == "Game", 300));
    a.tap(Button::B);
    b.tap(Button::B);
    for i in 0..400 {
        if a.top() == "Game" {
            a.tap(Button::B);
        }
        if b.top() == "Game" && i % 3 == 0 {
            b.tap(Button::B);
        }
        both(&mut a, &mut b, 3);
        if a.top() != "Game" && b.top() != "Game" {
            break;
        }
    }
    assert!(wait(&mut a, &mut b, |a, b| a.top() == "Message" && b.top() == "Message", 1500), "{} {}", a.top(), b.top());
    assert_eq!(a.core.game.friend_count(&idb), 2);
    a.back_to_main();
    b.back_to_main();

    // GIFT
    a.core.game.items.insert("CAKE".into(), 2);
    connect(&mut a, &mut b);
    a.taps("AA");
    a.tap(Button::B);
    assert_eq!(a.top(), "Pick");
    a.tap(Button::B);
    assert!(wait(&mut a, &mut b, |a, b| a.top() == "Message" && b.top() == "Message", 600));
    assert_eq!(a.core.game.items.get("CAKE"), Some(&1));
    assert_eq!(b.core.game.items.get("CAKE"), Some(&1));
    a.back_to_main();
    b.back_to_main();

    // MARRY before soulmate level: refused locally
    connect(&mut a, &mut b);
    a.taps("AAA");
    a.tap(Button::B);
    a.frames(2);
    assert_eq!(a.top(), "Message");
    a.back_to_main();
    b.back_to_main();
    both(&mut a, &mut b, 30);

    // MARRY as soulmates
    a.core.game.friends.get_mut(&idb).unwrap().count = 10;
    b.core.game.friends.get_mut(&ida).unwrap().count = 10;
    connect(&mut a, &mut b);
    a.taps("AAA");
    a.tap(Button::B);
    assert!(wait(&mut a, &mut b, |_, b| b.top() == "Confirm", 600), "no proposal");
    b.tap(Button::B);
    assert!(wait(&mut a, &mut b, |a, b| a.top() == "Wedding" && b.top() == "Wedding", 600));
    both(&mut a, &mut b, 30 * 7);
    assert_eq!(a.core.game.pet.gen, 2);
    assert_eq!(b.core.game.pet.gen, 2);
    assert_eq!(a.core.game.album.last().unwrap().partner.as_deref(), Some("TEST"));
}

#[test]
fn host_can_refuse_a_pairing() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut a = adult(21, "p_a");
    let mut b = adult(22, "g_b");
    a.core.click_icon(7);
    b.core.click_icon(7);
    a.tap(Button::B);
    b.taps("AA");
    b.tap(Button::B);
    b.core.text("127.0.0.1");
    b.core.key(bytepet::screens::Key::Enter);
    assert!(wait(&mut a, &mut b, |a, _| lcd_text(a).contains("CODE"), 600));
    a.tap(Button::C); // refuse
    assert!(wait(&mut a, &mut b, |_, b| b.top() == "Message", 600), "guest still {}", b.top());
    assert_eq!(b.core.game.friend_count(&a.core.game.device_id), 0);
}
