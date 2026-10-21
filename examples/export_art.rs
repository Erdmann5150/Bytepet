//! Renders device screenshots, LCD screens and every character sprite to PNG files.
//! Used for the README and the player's manual, and to eyeball changes to the art.
//!
//!     cargo run --example export_art --no-default-features -- <out dir>

use std::path::{Path, PathBuf};

use bytepet::content::{Content, Pose, Stage};
use bytepet::core::Core;
use bytepet::games::{self, GameHost, SoloGame, VersusGame};
use bytepet::overlay::Page;
use bytepet::render::{lcd_pixmap, sprite_pixmap, Renderer};
use bytepet::rng::Pcg32;
use bytepet::screens::{boxed, care, town, Screen};
use bytepet::sim::{Call, CallReason, Game};
use bytepet::Button;

struct Rig {
    core: Core,
    wall: f64,
}

impl Rig {
    fn new(stage: Stage, character: &str) -> Rig {
        let content = Content::builtin();
        let mut game = Game::new(&content, Pcg32::new(5));
        game.pet.name = "KIWI".into();
        game.pet.stage = stage;
        game.pet.character = character.into();
        game.pet.hunger = 4;
        game.pet.happy = 4;
        game.pet.t_poop = 1e9;
        game.pet.t_mail = 1e9;
        let mut core = Core::new(content, game);
        core.hour_at = |_| 12.0;
        core.ui.asked_name_for = core.game.pet.id;
        Rig { core, wall: 1_800_000_000.0 }
    }
    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.wall += 1.0 / 30.0;
            self.core.frame_at(1.0 / 30.0, self.wall);
        }
    }
    fn push(&mut self, s: Box<dyn Screen>) {
        self.core.stack.push(s);
    }
    fn tap(&mut self, b: Button) {
        self.core.press(b);
        self.frames(1);
        self.core.release(b);
        self.frames(1);
    }
    fn lcd(&mut self, dir: &Path, name: &str) {
        self.core.render_lcd();
        lcd_pixmap(&self.core.lcd, 10).save_png(dir.join(format!("lcd_{name}.png"))).unwrap();
    }
    fn device(&mut self, dir: &Path, name: &str, scale: f32) {
        // settle the LCD "ghosting" so screenshots are crisp
        self.core.shade = self.core.lcd.buf.map(|v| v as f32);
        let pm = Renderer::new().render(&self.core, scale, 1.0, self.wall);
        pm.save_png(dir.join(format!("{name}.png"))).unwrap();
    }
}

fn game_shot(dir: &Path, name: &str, g: Box<dyn games::MiniGame>, frames: usize, presses: &[Button]) {
    let mut r = Rig::new(Stage::Adult, "p_a");
    r.push(boxed(GameHost::new(g, 4)));
    r.frames(12);
    r.tap(Button::B); // skip the intro
    for i in 0..frames {
        if !presses.is_empty() && i % 9 == 0 {
            r.tap(presses[(i / 9) % presses.len()]);
        } else {
            r.frames(1);
        }
    }
    r.lcd(dir, &format!("game_{name}"));
}

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "art".into()));
    std::fs::create_dir_all(&dir).unwrap();
    let content = Content::builtin();

    // every character, every pose, plus the egg
    for ch in &content.characters {
        for (pose, tag) in [(Pose::Idle1, ""), (Pose::Happy, "_happy"), (Pose::Eat, "_eat"), (Pose::Sleep, "_sleep")] {
            sprite_pixmap(ch.frame(pose), 10, 1).save_png(dir.join(format!("char_{}{}.png", ch.id, tag))).unwrap();
        }
    }
    sprite_pixmap(content.prop("egg"), 10, 1).save_png(dir.join("char_egg.png")).unwrap();
    for (i, ic) in content.icons.iter().enumerate() {
        sprite_pixmap(ic, 8, 0).save_png(dir.join(format!("icon_{i}.png"))).unwrap();
    }

    // the title in the LCD font, for the manual's cover
    let mut lcd = bytepet::lcd::Lcd::default();
    lcd.text("BYTEPET", 0, 0);
    let cell = 12;
    let mut title = tiny_skia::Pixmap::new(27 * cell, 5 * cell).unwrap();
    let mut pink = tiny_skia::Paint::default();
    pink.set_color_rgba8(236, 72, 153, 255);
    for y in 0..5 {
        for x in 0..27 {
            if lcd.get(x, y) {
                let r = tiny_skia::Rect::from_xywh(
                    (x as u32 * cell) as f32,
                    (y as u32 * cell) as f32,
                    cell as f32,
                    cell as f32,
                );
                title.fill_rect(r.unwrap(), &pink, tiny_skia::Transform::identity(), None);
            }
        }
    }
    title.save_png(dir.join("title.png")).unwrap();

    // devices: every shell, and the three common sizes
    let mut r = Rig::new(Stage::Adult, "g_b");
    r.frames(30);
    for (i, p) in bytepet::look::PRESETS.iter().enumerate() {
        r.core.game.settings.set_shell(i as u32);
        r.frames(1);
        r.device(&dir, &format!("shell_{}", p.name.to_lowercase()), 0.5);
    }
    r.core.game.settings.set_shell(1);
    r.device(&dir, "device_cover", 2.0);
    r.core.game.settings.set_shell(0);
    r.core.game.pet.poop = 0;
    r.frames(1);
    r.device(&dir, "app_icon_src", 1.9);
    for (name, s) in [("small", 0.40), ("medium", 0.55), ("large", 0.75)] {
        r.core.game.settings.set_shell(0);
        r.device(&dir, &format!("size_{name}"), s);
    }
    // the settings, on the device's screen
    r.core.overlay.env.hide_label = "HIDE TO TRAY";
    r.core.overlay.open();
    r.core.overlay.hover = Some(1);
    r.device(&dir, "menu_open", 0.75);
    for (name, scale) in bytepet::overlay::SIZES {
        r.device(&dir, &format!("menu_{}", name.to_lowercase()), scale);
    }
    // the SHELL page and a part's page
    r.core.overlay.page = Page::Shell;
    r.core.overlay.hover = Some(1);
    r.device(&dir, "menu_shell", 0.75);
    r.core.overlay.page = Page::Part(bytepet::look::Part::Body);
    r.core.overlay.hover = Some(0);
    r.device(&dir, "menu_body", 0.75);
    r.core.overlay.open_help();
    r.device(&dir, "help_open", 0.75);
    r.device(&dir, "help_small", 0.40);
    r.core.overlay.next_help();
    r.device(&dir, "help2_small", 0.40);
    r.core.overlay.close();

    // LCD screens
    let mut r = Rig::new(Stage::Child, "child_a");
    r.frames(10);
    r.lcd(&dir, "main");
    r.core.game.pet.poop = 3;
    r.core.game.pet.sick = true;
    r.frames(2);
    r.lcd(&dir, "sick");
    let mut r = Rig::new(Stage::Teen, "teen_g");
    r.core.game.pet.asleep = true;
    r.frames(10);
    r.lcd(&dir, "sleep");
    let mut r = Rig::new(Stage::Teen, "teen_p");
    r.core.game.pet.call = Some(Call { reason: CallReason::Need, t: 0.0 });
    r.frames(10);
    r.lcd(&dir, "call");
    let mut r = Rig::new(Stage::Egg, "egg");
    r.frames(2);
    r.lcd(&dir, "egg");
    let mut r = Rig::new(Stage::Teen, "teen_h");
    r.core.game.pet.hunger = 3;
    r.core.game.pet.discipline = 50;
    r.push(boxed(care::Status::new()));
    r.tap(Button::A);
    r.tap(Button::A);
    r.lcd(&dir, "status_hunger");
    r.tap(Button::A);
    r.tap(Button::A);
    r.lcd(&dir, "status_train");
    let mut r = Rig::new(Stage::Child, "child_a");
    r.push(boxed(care::FoodMenu::new()));
    r.frames(2);
    r.lcd(&dir, "foodmenu");
    r.push(boxed(care::Eat::new(content.prop("meal").clone())));
    r.frames(25);
    r.lcd(&dir, "eat");
    let mut r = Rig::new(Stage::Child, "child_a");
    r.push(boxed(care::Clock::new()));
    r.frames(2);
    r.lcd(&dir, "clock");
    for state in ["host", "code", "visit"] {
        let mut r = Rig::new(Stage::Adult, "p_a");
        r.push(boxed(bytepet::link::LinkScreen::preview(state, "g_b")));
        r.core.render_lcd();
        r.lcd(&dir, &format!("link_{state}"));
    }
    let mut r = Rig::new(Stage::Adult, "h_a");
    r.push(boxed(town::TownMenu::new()));
    r.frames(2);
    r.lcd(&dir, "town");
    let mut r = Rig::new(Stage::Adult, "p_b");
    r.push(boxed(care::Evolve::new("p_b".into())));
    r.frames(80);
    r.lcd(&dir, "evolve");
    let mut r = Rig::new(Stage::Adult, "h_a");
    r.push(boxed(care::Wedding::new("h_a".into(), "g_a".into())));
    r.frames(30);
    r.lcd(&dir, "wedding");
    let mut r = Rig::new(Stage::Adult, "h_a");
    r.push(boxed(care::Gone::new("child_a".into())));
    r.frames(100);
    r.lcd(&dir, "gone");
    let mut r = Rig::new(Stage::Egg, "egg");
    r.core.game.pet.name.clear();
    r.core.ui.asked_name_for = 0;
    r.frames(2);
    for ch in "KIWI".chars() {
        r.core.text(&ch.to_string());
    }
    r.tap(Button::A);
    r.tap(Button::A);
    r.tap(Button::A);
    r.tap(Button::A);
    r.lcd(&dir, "name");

    // games
    game_shot(&dir, "climb", SoloGame::Climb.make(), 95, &[]);
    game_shot(&dir, "tug", SoloGame::Tug.make(), 30, &[Button::B]);
    game_shot(&dir, "apples", SoloGame::Apples.make(), 70, &[]);
    game_shot(&dir, "manhole", SoloGame::Manhole.make(), 100, &[]);
    game_shot(&dir, "quiz", Box::new(games::Quiz::new("CLASS", 5)), 5, &[]);
    game_shot(&dir, "office", Box::new(games::Office::default()), 40, &[]);
    game_shot(&dir, "chef", Box::new(games::Chef::default()), 5, &[]);
    game_shot(&dir, "fire", Box::new(games::Fire::default()), 110, &[]);
    game_shot(&dir, "rock", Box::new(games::Rock::default()), 60, &[]);
    game_shot(&dir, "science", Box::new(games::Science::default()), 14, &[]);
    game_shot(&dir, "eats", VersusGame::Eats.make(), 30, &[Button::B]);
    game_shot(&dir, "flags", VersusGame::Flags.make(), 5, &[]);
    game_shot(&dir, "bump", VersusGame::Bump.make(), 12, &[]);
    println!("wrote art to {}", dir.display());
}
