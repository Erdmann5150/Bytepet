//! End-to-end flows through the real screens, driven by button presses.

mod common;

use bytepet::content::Stage;
use bytepet::sim::{Call, CallReason, Mail, MailKind};
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
fn bedtime_can_be_shifted_for_night_owls() {
    let mut r = Rig::with_pet(9, Stage::Child, "child_a");
    r.frames(1);
    assert!(!r.core.game.pet.asleep, "the rig's clock says noon");
    r.core.click_icon(8);
    r.taps("AAAAAA"); // SETUP
    r.tap(Button::B);
    r.taps("AAAA"); // BEDTIME
    for _ in 0..12 {
        r.tap(Button::B);
    }
    assert_eq!(r.core.game.settings.bedtime_shift, 12);
    // a child's 20:00-08:00 night is now 08:00-20:00: it's noon, so bedtime
    assert!(r.core.game.pet.asleep);
}

#[test]
fn a_menu_left_alone_goes_home() {
    let mut r = Rig::with_pet(9, Stage::Child, "child_a");
    r.core.click_icon(8);
    r.tap(Button::B); // MAIL: "NO MAIL", then back to town
    r.tap(Button::B);
    assert_eq!(r.top(), "TownMenu");
    r.frames(30 * 9);
    assert_eq!(r.top(), "TownMenu", "too soon");
    r.frames(30 * 2);
    assert!(r.core.at_home());
    // the icon cursor stays lit until then, and goes out with the menu
    assert_eq!(r.core.ui.sel, -1);
    // at home, a highlighted icon goes out after a while on its own
    r.tap(Button::A);
    assert_eq!(r.core.ui.sel, 0);
    r.frames(30 * 9);
    assert_eq!(r.core.ui.sel, 0);
    r.frames(30 * 2);
    assert_eq!(r.core.ui.sel, -1);
    // the settings close too, but a press keeps them open (opening them from the tray counts
    // as a press, see the desktop shell)
    r.core.overlay.open();
    r.core.touch();
    r.frames(30 * 8);
    r.tap(Button::A);
    r.frames(30 * 8);
    assert!(r.core.overlay.is_open());
    r.frames(30 * 3);
    assert!(!r.core.overlay.is_open());
    // a game is never cut short, nor its score
    let mut r = Rig::with_pet(11, Stage::Teen, "teen_g");
    r.core.click_icon(3);
    r.tap(Button::B);
    assert_eq!(r.top(), "Game");
    r.frames(30 * 61);
    assert!(!r.core.at_home(), "{:?}", r.core.stack_names());
}

#[test]
fn the_alert_icon_shows_every_need() {
    let mut r = Rig::with_pet(16, Stage::Child, "child_a");
    let alert = |r: &Rig| r.core.icon_states(0.0)[9];
    r.frames(1);
    assert!(!alert(&r), "a happy, clean pet needs nothing");
    r.core.game.pet.poop = 1;
    assert!(alert(&r), "poop");
    r.core.click_icon(2);
    assert!(r.until("Main", 90));
    assert!(!alert(&r));
    r.core.game.pet.sick = true;
    assert!(alert(&r), "sick");
    r.core.game.pet.sick = false;
    r.core.game.pet.happy = 0;
    assert!(alert(&r), "an empty heart");
    r.core.game.pet.happy = 4;
    r.frames(1);
    assert!(!alert(&r));
    // falling asleep with the lights on, it calls about them, and the alert shows it...
    r.core.hour_at = |_| 23.0;
    r.frames(1);
    assert!(r.core.game.pet.asleep && r.core.game.pet.lights);
    assert!(alert(&r), "the lights call");
    // ...until the call times out (a care mistake): the lights left on aren't a need all night
    r.core.game.settings.speed = 60;
    let mistakes = r.core.game.pet.mistakes;
    r.frames(30 * 20);
    assert_eq!(r.core.game.pet.mistakes, mistakes + 1);
    assert!(r.core.game.pet.asleep && r.core.game.pet.lights);
    assert!(!alert(&r), "lit all night");
    // and hearts can't be filled while it sleeps, so they don't count either
    r.core.game.pet.hunger = 0;
    assert!(!alert(&r), "hearts can't be filled while it sleeps");
}

#[test]
fn turning_the_lights_off_answers_the_alert() {
    let mut r = Rig::with_pet(19, Stage::Child, "child_a");
    let alert = |r: &Rig| r.core.icon_states(0.0)[9];
    r.core.hour_at = |_| 23.0;
    r.frames(1);
    assert!(alert(&r));
    r.core.click_icon(4); // lights
    r.frames(30);
    assert!(!r.core.game.pet.lights);
    assert!(!alert(&r));
}

#[test]
fn the_tray_tooltip_says_how_the_pet_is() {
    let mut r = Rig::with_pet(18, Stage::Child, "child_a");
    r.core.game.pet.age_s = 2.5 * 86400.0;
    assert_eq!(r.core.summary(), "Test the child, 2 days - hunger 4/4, happy 4/4\nAll good");
    r.core.game.pet.hunger = 0;
    r.core.game.pet.poop = 2;
    r.core.game.pet.sick = true;
    assert!(r.core.summary().ends_with("\nNeeds food, cleaning, medicine"), "{}", r.core.summary());
    // asleep, calling about the lights
    r.core.game.pet.asleep = true;
    r.core.game.pet.call = Some(Call { reason: CallReason::Lights, t: 0.0 });
    assert!(r.core.summary().ends_with("\nNeeds cleaning, medicine, lights off"), "{}", r.core.summary());
    // the call is over: the lights left on are no longer a need
    r.core.game.pet.call = None;
    assert!(r.core.summary().ends_with("\nNeeds cleaning, medicine"), "{}", r.core.summary());
    r.core.game.pet = Default::default();
    assert_eq!(r.core.summary(), "An egg, waiting to hatch");
}

#[test]
fn a_still_screen_is_not_redrawn() {
    use bytepet::render::FrameKey;
    let mut r = Rig::with_pet(17, Stage::Child, "child_a");
    let key = |r: &Rig| FrameKey::new(&r.core, 0.55, r.wall);
    // lights off: the screen is dark and nothing moves, so once the dots have faded in, the
    // frame stops changing
    r.core.click_icon(4);
    r.frames(60);
    let still = key(&r);
    r.frames(30);
    assert_eq!(key(&r), still);
    // a press changes it
    r.core.press(Button::A);
    assert_ne!(key(&r), still);
    r.core.release(Button::A);
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
fn right_click_on_the_home_screen_is_c_not_the_settings() {
    let mut r = Rig::with_pet(12, Stage::Child, "child_a");
    r.core.click_icon(1); // food
    r.core.right_click(); // back out of the food menu
    r.frames(1);
    assert!(r.core.at_home());
    assert_eq!(r.core.ui.sel, 1);
    r.core.right_click(); // clears the highlighted icon
    assert_eq!(r.core.ui.sel, -1);
    r.core.right_click(); // nothing selected: still just C, the settings stay shut
    r.frames(1);
    assert!(!r.core.overlay.is_open());
    assert!(r.core.at_home());
    assert_eq!(r.core.ui.sel, -1);
    // once open (Town > SETUP > MORE, or the tray), right-click closes them
    r.core.overlay.open();
    r.core.right_click();
    assert!(!r.core.overlay.is_open());
}

#[test]
fn clicked_one_shot_icons_light_only_while_they_act() {
    let mut r = Rig::with_pet(14, Stage::Child, "child_a");
    let lit = |r: &Rig| r.core.icon_states(0.0);
    // clean: lit for the whole flush, out once it's over
    r.core.click_icon(2);
    r.frames(1);
    assert_eq!(r.top(), "Flush");
    assert!(lit(&r)[2]);
    assert!(r.until("Main", 90));
    r.frames(1);
    assert!(!lit(&r)[2]);
    assert_eq!(r.core.ui.sel, -1);
    // lights: a brief flash, as there's no animation
    r.core.click_icon(4);
    r.frames(1);
    assert!(lit(&r)[4]);
    r.frames(15);
    assert!(!lit(&r)[4]);
    r.core.click_icon(4);
    // medicine when not sick: lit through the head shake
    r.core.click_icon(5);
    r.frames(1);
    assert!(lit(&r)[5]);
    assert!(r.until("Main", 150));
    r.frames(1);
    assert!(!lit(&r)[5]);
    // a menu icon stays lit after its menu closes
    r.core.click_icon(1);
    r.tap(Button::C);
    assert!(r.core.at_home());
    r.frames(30);
    assert!(lit(&r)[1]);
    // and the buttons leave the cursor where they put it
    r.tap(Button::A); // from food to clean
    r.tap(Button::B);
    assert_eq!(r.top(), "Flush");
    assert!(r.until("Main", 90));
    r.frames(30);
    assert_eq!(r.core.ui.sel, 2);
}

#[test]
fn the_flush_wave_crosses_the_home_screen_as_it_is() {
    let mut r = Rig::with_pet(15, Stage::Child, "child_a");
    r.frames(1);
    // three poops corner the pet at the left
    r.core.game.pet.poop = 3;
    r.frames(1);
    let home = r.core.lcd.clone();
    r.core.click_icon(2);
    r.frames(1);
    assert_eq!(r.top(), "Flush");
    // the pet stays where it was, in the same pose, ahead of the wave
    for x in 0..16 {
        for y in 0..16 {
            assert_eq!(r.core.lcd.get(x, y), home.get(x, y), "({x}, {y}) changed when the flush started");
        }
    }
    assert!(r.until("Main", 90));
    assert_eq!(r.core.game.pet.poop, 0);
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

#[test]
fn the_device_shakes_with_the_call_beep() {
    use bytepet::core::SHAKE_REACH;
    let mut r = Rig::with_pet(12, Stage::Child, "child_a");
    r.frames(5);
    assert_eq!(r.core.shake_offset(), 0.0);
    // hungry: it calls
    r.core.game.pet.hunger = 0;
    let mut most = 0.0f32;
    for _ in 0..10 {
        r.frames(1);
        most = most.max(r.core.shake_offset().abs());
    }
    assert!(most > 0.5 && most <= SHAKE_REACH as f32, "shook {most}");
    // a quick buzz, then still again
    r.frames(30);
    assert_eq!(r.core.shake_offset(), 0.0);
    // and again with the reminder beep a minute later, sound or no sound
    r.core.game.settings.muted = true;
    let mut shook = false;
    for _ in 0..30 * 61 {
        r.frames(1);
        shook |= r.core.shake_offset() != 0.0;
    }
    assert!(shook, "no shake with the reminder beep");
}
