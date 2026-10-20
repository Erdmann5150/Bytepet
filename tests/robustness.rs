//! Random input against every screen, plus rendering checks.

mod common;

use bytepet::content::Stage;
use bytepet::render::Renderer;
use bytepet::rng::Pcg32;
use bytepet::Button;
use common::Rig;

/// Mash random buttons at high speed through the whole life cycle. Nothing may panic.
fn fuzz(seed: u64, frames: usize) -> Vec<&'static str> {
    let mut r = Rig::new(seed);
    r.core.game.settings.speed = 600;
    r.core.game.gp = 5000;
    let mut rng = Pcg32::new(seed * 7 + 1);
    let mut seen = Vec::new();
    for i in 0..frames {
        // keep the network out of the fuzz: leave the link screen straight away
        if r.top() == "Link" {
            r.tap(Button::C);
        }
        if rng.chance(0.25) {
            let b = [Button::A, Button::A, Button::B, Button::B, Button::C][rng.below(5)];
            r.core.press(b);
            r.core.release(b);
        }
        if rng.chance(0.01) {
            r.core.click_icon(rng.below(9));
        }
        if r.core.wants_text() && rng.chance(0.05) {
            r.core.key(bytepet::screens::Key::Enter);
        }
        // vary the time of day so sleeping is covered too
        if i % 3000 == 0 {
            r.core.hour_at = if (i / 3000) % 2 == 0 { |_| 12.0 } else { |_| 23.0 };
        }
        r.frames(1);
        let t = r.top();
        if !seen.contains(&t) {
            seen.push(t);
        }
    }
    seen
}

#[test]
fn fuzz_short() {
    for seed in 0..3 {
        let seen = fuzz(seed, 20_000);
        assert!(seen.len() > 8, "only reached {seen:?}");
    }
}

#[test]
#[ignore = "slow: cargo test -- --ignored"]
fn fuzz_long() {
    let mut all = Vec::new();
    for seed in 0..12 {
        for s in fuzz(seed, 120_000) {
            if !all.contains(&s) {
                all.push(s);
            }
        }
    }
    println!("screens reached: {all:?}");
}

#[test]
fn device_render_has_see_through_corners() {
    let mut r = Rig::with_pet(1, Stage::Adult, "g_b");
    r.frames(5);
    for &scale in &[0.4f32, 0.55, 1.0] {
        let pm = Renderer::new().render(&r.core, scale, 1.0, None, r.wall);
        let (w, h) = (pm.width(), pm.height());
        let alpha = |x: u32, y: u32| pm.pixel(x, y).unwrap().alpha();
        assert_eq!(alpha(1, h - 2), 0, "bottom-left corner should be transparent");
        assert_eq!(alpha(w - 2, 1), 0, "top-right corner should be transparent");
        assert_eq!(alpha(w / 2, h / 2), 255, "the device itself should be opaque");
    }
}

#[test]
fn every_shell_and_the_menu_render() {
    let mut r = Rig::with_pet(2, Stage::Teen, "teen_p");
    r.frames(3);
    let env = bytepet::overlay::Env { login: false, hide_label: "MINIMIZE" };
    for shell in 0..8 {
        r.core.game.settings.shell = shell;
        let pm = Renderer::new().render(&r.core, 0.55, 1.0, Some(&env), r.wall);
        assert!(pm.width() > 100);
    }
    r.core.overlay.open();
    let pm = Renderer::new().render(&r.core, 0.4, 1.0, Some(&env), r.wall);
    // the menu panel is drawn over the middle of the device
    assert!(pm.pixel(pm.width() / 2, pm.height() / 2).unwrap().alpha() > 200);
}

#[test]
fn a_damaged_save_is_kept_and_a_new_game_starts() {
    let dir = std::env::temp_dir().join(format!("bytepet-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("save.json");
    std::fs::write(&path, "{ this is not json").unwrap();
    let core = bytepet::core::Core::open(bytepet::content::Content::builtin(), path.clone(), false);
    assert_eq!(core.game.pet.stage, Stage::Egg);
    assert!(dir.join("save.json.corrupt").exists());
    assert!(!core.startup_notes.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
