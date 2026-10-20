//! End-to-end flows through the real screens, driven by button presses.

mod common;

use bytepet::content::Stage;
use bytepet::sim::{Mail, MailKind};
use bytepet::Button;
use common::Rig;

#[test]
fn first_run_name_and_hatch() {
    let mut r = Rig::new(1);
    r.frames(2);
    assert_eq!(r.top(), "NameEntry");
    assert!(r.core.wants_text());
    r.core.text("mochi");
    r.core.key(bytepet::screens::Key::Enter);
    r.frames(2);
    assert_eq!(r.core.game.pet.name, "MOCHI");
    // the egg hatches after a minute
    r.frames(30 * 62);
    assert_eq!(r.core.game.pet.stage, Stage::Baby);
    assert!(r.until("Hatch", 30));
    r.frames(30 * 4);
    assert_eq!(r.top(), "Main");
}

#[test]
fn feed_through_the_food_menu() {
    let mut r = Rig::with_pet(2, Stage::Child, "child_a");
    r.core.game.pet.hunger = 1;
    r.core.game.pet.discipline = 100; // never refuses
    r.core.click_icon(1);
    assert_eq!(r.top(), "FoodMenu");
    r.tap(Button::B);
    assert_eq!(r.top(), "Eat");
    r.frames(30 * 3);
    assert_eq!(r.top(), "FoodMenu");
    assert_eq!(r.core.game.pet.hunger, 2);
}

#[test]
fn a_solo_game_pays_out_once() {
    let mut r = Rig::with_pet(3, Stage::Teen, "teen_g");
    r.core.click_icon(3);
    assert_eq!(r.top(), "GameMenu");
    r.taps("A"); // TUG
    r.tap(Button::B);
    assert_eq!(r.top(), "Game");
    r.tap(Button::B); // skip the intro
    for _ in 0..200 {
        r.tap(Button::B); // mash
    }
    assert!(r.until("React", 30 * 15), "top is {}", r.top());
    let skill = r.core.game.pet.skills.gorgeous;
    assert!(skill > 0);
    r.frames(30 * 3);
    assert_eq!(r.top(), "Message");
    r.back_to_main();
    assert_eq!(r.core.game.pet.skills.gorgeous, skill, "results applied twice");
}

#[test]
fn school_letter_flow() {
    let mut r = Rig::with_pet(4, Stage::Child, "child_a");
    let t = r.core.game.pet.t;
    r.core.game.pet.mail.push(Mail { kind: MailKind::School, t, gp: None, item: None, job: None });
    r.core.click_icon(8);
    r.tap(Button::B); // MAIL
    assert_eq!(r.top(), "Confirm");
    r.tap(Button::B); // go
    assert_eq!(r.top(), "Pick");
    r.taps("AA"); // passion teacher
    r.tap(Button::B);
    assert_eq!(r.top(), "Game");
    r.tap(Button::B);
    // answer every question with A; about half will be right
    for _ in 0..400 {
        if r.top() != "Game" {
            break;
        }
        r.tap(Button::A);
        r.frames(20);
    }
    assert_eq!(r.top(), "Message");
    r.back_to_main();
}

#[test]
fn job_interview_and_work() {
    let mut r = Rig::with_pet(5, Stage::Adult, "p_a");
    r.core.game.pet.skills.passion = 60;
    let t = r.core.game.pet.t;
    r.core.game.pet.mail.push(Mail { kind: MailKind::Job, t, gp: None, item: None, job: Some("CHEF".into()) });
    r.core.click_icon(8);
    r.tap(Button::B);
    r.taps("BB"); // read the offer
    assert_eq!(r.top(), "Confirm");
    r.tap(Button::B);
    assert_eq!(r.top(), "Game");
    // cheat a little: answer by peeking is not possible, so retry until hired
    for attempt in 0..20 {
        r.tap(Button::B);
        for _ in 0..200 {
            if r.top() != "Game" {
                break;
            }
            r.tap(if attempt % 2 == 0 { Button::A } else { Button::C });
            r.frames(20);
        }
        if r.core.game.pet.job.is_some() {
            break;
        }
        r.back_to_main();
        let t = r.core.game.pet.t;
        r.core.game.pet.mail.push(Mail { kind: MailKind::Job, t, gp: None, item: None, job: Some("CHEF".into()) });
        r.core.click_icon(8);
        r.tap(Button::B);
        r.taps("BB");
        r.tap(Button::B);
    }
    assert_eq!(r.core.game.pet.job.as_deref(), Some("CHEF"));
    r.back_to_main();
    let gp = r.core.game.gp;
    r.core.click_icon(3);
    r.taps("AAAA"); // WORK
    r.tap(Button::B);
    r.taps("BB"); // briefing
    assert_eq!(r.top(), "Game");
    r.tap(Button::B);
    for _ in 0..30 * 35 {
        if r.top() != "Game" {
            break;
        }
        r.frames(1);
    }
    assert!(r.until("Message", 60));
    assert!(r.core.game.gp > gp, "no pay");
}

#[test]
fn matchmaker_wedding_starts_generation_two() {
    let mut r = Rig::with_pet(6, Stage::Adult, "h_a");
    r.core.game.pet.age_s = 6.0 * 86400.0;
    let first_id = r.core.game.pet.id;
    r.core.click_icon(8);
    r.taps("AAA"); // PARTNER
    r.tap(Button::B);
    r.tap(Button::B); // greeting
    assert_eq!(r.top(), "Pick");
    r.tap(Button::B);
    assert_eq!(r.top(), "Confirm");
    r.tap(Button::B);
    assert_eq!(r.top(), "Wedding");
    r.frames(30 * 7);
    assert_eq!(r.core.game.pet.gen, 2);
    assert_eq!(r.core.game.pet.stage, Stage::Egg);
    assert_ne!(r.core.game.pet.id, first_id);
    assert_eq!(r.core.game.album.last().unwrap().ending, "MARRIED");
}

#[test]
fn a_result_for_a_departed_pet_is_ignored() {
    let mut r = Rig::with_pet(7, Stage::Teen, "teen_h");
    r.core.click_icon(3);
    r.tap(Button::B); // CLIMB
    r.tap(Button::B); // start
                      // the pet leaves mid-game
    r.core.game.pet.stage = Stage::Gone;
    let gp = r.core.game.gp;
    for _ in 0..30 * 40 {
        r.frames(1);
    }
    assert_eq!(r.core.game.gp, gp);
    assert_eq!(r.core.game.pet.skills.humor, 0);
}

#[test]
fn lights_and_sleep_from_the_main_screen() {
    let mut r = Rig::with_pet(8, Stage::Child, "child_a");
    r.core.hour_at = |_| 21.0;
    r.frames(2);
    assert!(r.core.game.pet.asleep);
    r.core.click_icon(4);
    assert!(!r.core.game.pet.lights);
    assert!(r.core.game.pet.call.is_none());
}

#[test]
fn time_can_be_paused_while_closed() {
    let mut r = Rig::with_pet(9, Stage::Child, "child_a");
    r.core.click_icon(8);
    r.taps("AAAAAA"); // SETUP
    r.tap(Button::B);
    assert_eq!(r.top(), "Setup");
    r.taps("AAA"); // WHEN OFF
    r.tap(Button::B);
    assert!(!r.core.game.settings.run_away);
    // a long gap (the app was closed) no longer ages the pet
    let age = r.core.game.pet.age_s;
    r.wall += 3.0 * 3600.0;
    r.frames(1);
    assert!(r.core.game.pet.age_s - age < 5.0, "aged {}s", r.core.game.pet.age_s - age);
}

#[test]
fn clicking_an_icon_switches_menus() {
    let mut r = Rig::with_pet(10, Stage::Child, "child_a");
    r.core.click_icon(1); // food
    assert_eq!(r.top(), "FoodMenu");
    r.core.click_icon(8); // town, straight from the food menu
    assert_eq!(r.top(), "TownMenu");
    r.tap(Button::A); // SHOP
    r.tap(Button::B);
    assert_eq!(r.top(), "Shop");
    r.core.click_icon(0); // status, from two menus deep
    assert_eq!(r.top(), "Status");
    assert_eq!(r.core.stack_names(), vec!["Main", "Status"]);
    // right-click is C inside menus
    r.core.right_click();
    r.frames(1);
    assert!(r.core.at_home());
}

#[test]
fn scroll_wheel_moves_through_menus_both_ways() {
    let mut r = Rig::with_pet(13, Stage::Child, "child_a");
    // home: up from nothing selected lands on the last icon, down wraps to the first
    r.core.scroll(true);
    assert_eq!(r.core.ui.sel, 8);
    r.core.scroll(false);
    assert_eq!(r.core.ui.sel, 0);
    r.core.scroll(true);
    assert_eq!(r.core.ui.sel, 8);
    // town: up wraps to the end, then down twice and up once ends on SHOP
    r.core.click_icon(8);
    r.core.scroll(true);
    r.core.scroll(false);
    r.core.scroll(false);
    r.core.scroll(false);
    r.core.scroll(true);
    r.tap(Button::B);
    assert_eq!(r.top(), "Shop");
}

#[test]
fn right_click_clears_the_selection_before_opening_settings() {
    let mut r = Rig::with_pet(12, Stage::Child, "child_a");
    r.core.click_icon(1); // food
    r.core.right_click(); // back out of the food menu
    r.frames(1);
    assert!(r.core.at_home());
    assert_eq!(r.core.ui.sel, 1);
    r.core.right_click(); // clears the highlighted icon, no settings menu yet
    assert_eq!(r.core.ui.sel, -1);
    assert!(!r.core.overlay.is_open());
    r.core.right_click(); // nothing selected: now it's the settings menu
    assert!(r.core.overlay.is_open());
    r.core.right_click(); // and right-click closes it again
    assert!(!r.core.overlay.is_open());
    // an icon picked with A is cleared the same way
    r.tap(Button::A);
    r.core.right_click();
    assert!(!r.core.overlay.is_open());
    assert_eq!(r.core.ui.sel, -1);
}

#[test]
fn icons_dont_interrupt_a_game() {
    let mut r = Rig::with_pet(11, Stage::Teen, "teen_g");
    r.core.click_icon(3);
    r.tap(Button::B); // CLIMB
    assert_eq!(r.top(), "Game");
    r.core.click_icon(1);
    assert_eq!(r.top(), "Game");
}
