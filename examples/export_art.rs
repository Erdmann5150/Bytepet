//! Renders device screenshots, LCD screens and every character sprite to PNG files.
//! Used for the README and the player's manual, and to eyeball changes to the art.
//!
//!     cargo run --example export_art --no-default-features -- <out dir>

use std::path::{Path, PathBuf};

use bytepet::content::{Content, Pose, Stage};
use bytepet::core::Core;
use bytepet::games::{self, GameHost, SoloGame, VersusGame};
use bytepet::look::{Look, PRESETS};
use bytepet::overlay::Page;
use bytepet::render::{lcd_pixmap, sprite_pixmap, Renderer};
use bytepet::rng::Pcg32;
use bytepet::screens::{boxed, care, town, Screen};
use bytepet::sim::{Call, CallReason, Game};
use bytepet::Button;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

struct Rig {
    core: Core,
    wall: f64,
}

/// A pet for each preset shell, in `PRESETS` order: BERRY LIME OCEAN GRAPE ZEBRA FLAME /
/// DONUT CAMO COSMIC SWEET MINT RETRO.
const SHELL_PETS: [(Stage, &str); 12] = [
    (Stage::Child, "child_a"),
    (Stage::Teen, "teen_h"),
    (Stage::Adult, "g_a"),
    (Stage::Adult, "s_m"),
    (Stage::Adult, "h_b"),
    (Stage::Adult, "p_a"),
    (Stage::Baby, "baby"),
    (Stage::Child, "child_b"),
    (Stage::Adult, "s_h"),
    (Stage::Adult, "g_b"),
    (Stage::Teen, "teen_p"),
    (Stage::Senior, "senior"),
];

impl Rig {
    fn new(stage: Stage, character: &str) -> Rig {
        Rig::seeded(stage, character, 5)
    }
    /// Pets with different seeds wander differently (see `wander`).
    fn seeded(stage: Stage, character: &str, seed: u64) -> Rig {
        let content = Content::builtin();
        let mut game = Game::new(&content, Pcg32::new(seed));
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
    /// Let the pet walk about the screen for a while, so a picture catches it mid-stroll
    /// rather than where every pet starts.
    fn wander(&mut self, secs: f64) {
        self.frames((secs * 30.0) as usize);
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

/// A device in `look` with a pet on it, after the pet has wandered about for a few seconds
/// (each `seed` walks its own way), so pictures don't all show it in the same spot. The key
/// ring is always on, so it isn't missing from some pictures for no clear reason.
fn shot(look: Look, (stage, character): (Stage, &str), seed: u64, scale: f32, tweak: impl FnOnce(&mut Rig)) -> Pixmap {
    let mut r = Rig::seeded(stage, character, seed);
    r.core.game.settings.look = Look { ring: true, ..look };
    tweak(&mut r);
    // an egg hatches after a minute, and doesn't walk anyway
    r.wander(if stage == Stage::Egg { 1.5 } else { 5.0 + (seed % 7) as f64 });
    r.core.shade = r.core.lcd.buf.map(|v| v as f32);
    Renderer::new().render(&r.core, scale, 1.0, r.wall)
}

/// Made-up looks that between them use every antenna, button shape, corner style and LCD
/// colour, including the ones no preset has: no antenna, star buttons and soft corners.
fn showcase() -> [Look; 6] {
    use bytepet::look::{Antenna as A, ButtonShape as B, Corners as C, Lcd as L, Pattern as P, PALETTE};
    let c = |i: usize| PALETTE[i].1;
    let look = |body, pattern, buttons, button_shape, antenna, corners, lcd| Look {
        pattern,
        buttons: Some(buttons),
        button_shape,
        antenna,
        antenna_color: Some(buttons),
        corners,
        lcd,
        ..Look::plain(body)
    };
    [
        look(c(12), P::Stripes, c(3), B::Star, A::None, C::Soft, L::Gray), // purple, yellow stars
        look(c(8), P::Dots, c(4), B::Gem, A::Star, C::Square, L::Amber),   // teal, gold
        look(c(10), P::Hearts, c(0), B::Heart, A::Heart, C::Round, L::Pink), // sky, pink hearts
        look(c(3), P::Zebra, c(19), B::Square, A::Bolt, C::Soft, L::Classic), // yellow and black
        look(c(11), P::Sprinkles, c(17), B::Round, A::Moon, C::Round, L::Blue), // navy, silver
        look(c(5), P::Camo, c(2), B::Star, A::Ball, C::Square, L::Classic), // lime, orange
    ]
}

/// Lay devices side by side, overlapping by `overlap` of the narrower of each pair and centred
/// on one line, the middle ones in front.
fn row(devices: &[Pixmap], overlap: f32) -> Pixmap {
    let mut xs = vec![0u32];
    for pair in devices.windows(2) {
        let o = (pair[0].width().min(pair[1].width()) as f32 * overlap) as u32;
        xs.push(xs.last().unwrap() + pair[0].width() - o);
    }
    let w = xs.last().unwrap() + devices.last().unwrap().width();
    let h = devices.iter().map(|d| d.height()).max().unwrap();
    let mut out = Pixmap::new(w, h).unwrap();
    // from the edges in, so each one stands in front of the ones further out
    let mid = (devices.len() - 1) as f32 / 2.0;
    let mut order: Vec<usize> = (0..devices.len()).collect();
    order.sort_by(|&a, &b| (b as f32 - mid).abs().total_cmp(&(a as f32 - mid).abs()));
    for i in order {
        let d = &devices[i];
        let y = ((h - d.height()) / 2) as i32;
        out.draw_pixmap(xs[i] as i32, y, d.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    out
}

/// The README's top image: a group shot of seven devices, each a different shell with a pet
/// at a different stage of life, egg to senior. The middle one is biggest and in front, and
/// the rest step down in size towards the edges. Between them they show every antenna, button
/// shape and corner style.
fn hero(dir: &Path) {
    let preset = |name: &str| PRESETS.iter().find(|p| p.name == name).map(|p| p.look).unwrap();
    // left to right, a life from egg to senior; sizes rise to the middle and fall again
    let cast = [
        (preset("FLAME"), (Stage::Egg, "egg"), 0.40),
        (preset("SWEET"), (Stage::Baby, "baby"), 0.48),
        (preset("MINT"), (Stage::Child, "child_a"), 0.56),
        (preset("COSMIC"), (Stage::Teen, "teen_g"), 0.66),
        (showcase()[0], (Stage::Adult, "s_p"), 0.56),
        (preset("ZEBRA"), (Stage::Adult, "h_a"), 0.48),
        (preset("RETRO"), (Stage::Senior, "senior"), 0.40),
    ];
    let devices: Vec<Pixmap> = cast
        .into_iter()
        .enumerate()
        .map(|(i, (look, pet, scale))| {
            shot(look, pet, 11 + i as u64, scale, |r| {
                let p = &mut r.core.game.pet;
                match i {
                    0 => p.stage_s = 40.0, // old enough to wobble
                    2 => p.poop = 2,
                    4 => p.praise_t = 100.0, // showing off
                    _ => {}
                }
            })
        })
        .collect();
    row(&devices, 0.2).save_png(dir.join("hero.png")).unwrap();
}

/// The README's "Make it yours" image: the twelve preset shells in order, then two designs of
/// your own (with the shapes no preset has), each with a different pet, in two staggered rows
/// of seven.
fn shells(dir: &Path) {
    let looks = PRESETS.iter().map(|p| p.look).chain([showcase()[0], showcase()[3]]);
    let pets = SHELL_PETS.into_iter().chain([(Stage::Adult, "g_c"), (Stage::Adult, "p_b")]);
    let devices: Vec<Pixmap> = looks
        .zip(pets)
        .enumerate()
        .map(|(i, (look, pet))| {
            shot(look, pet, 31 + i as u64, 0.53, |r| {
                if i % 4 == 3 {
                    r.core.game.pet.praise_t = 100.0; // a few of them dancing
                }
            })
        })
        .collect();
    let (dw, dh) = (devices[0].width(), devices[0].height());
    let per_row = devices.len() / 2;
    // a little closer than side by side, the second row half a step along; the rows don't
    // overlap, so every antenna shows in full
    let step_x = dw * 92 / 100;
    let w = step_x * (per_row as u32 - 1) + step_x / 2 + dw;
    let mut out = Pixmap::new(w, 2 * dh).unwrap();
    for (i, d) in devices.iter().enumerate() {
        let (r, c) = ((i / per_row) as u32, (i % per_row) as u32);
        let (x, y) = (c * step_x + r * step_x / 2, r * dh);
        out.draw_pixmap(x as i32, y as i32, d.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    out.save_png(dir.join("shells.png")).unwrap();
}

/// The manual's strip of designs of your own: every antenna, button shape, corner style and
/// LCD colour.
fn customs(dir: &Path) {
    let pets = [
        (Stage::Adult, "g_c"),
        (Stage::Adult, "h_c"),
        (Stage::Teen, "teen_h"),
        (Stage::Adult, "p_b"),
        (Stage::Adult, "s_g"),
        (Stage::Child, "child_b"),
    ];
    let devices: Vec<Pixmap> = showcase()
        .into_iter()
        .zip(pets)
        .enumerate()
        .map(|(i, (look, pet))| shot(look, pet, 51 + i as u64, 0.5, |_| {}))
        .collect();
    row(&devices, 0.06).save_png(dir.join("customs.png")).unwrap();
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

    hero(&dir);
    shells(&dir);
    customs(&dir);

    // devices: every shell, each with its own pet, and the three common sizes
    for (i, (p, pet)) in PRESETS.iter().zip(SHELL_PETS).enumerate() {
        let pm = shot(p.look, pet, 31 + i as u64, 0.5, |_| {});
        pm.save_png(dir.join(format!("shell_{}.png", p.name.to_lowercase()))).unwrap();
    }
    let mut r = Rig::new(Stage::Adult, "g_b");
    r.frames(30);
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
    let mut r = Rig::seeded(Stage::Child, "child_a", 7);
    r.wander(6.0);
    r.lcd(&dir, "main");
    r.core.game.pet.poop = 3;
    r.core.game.pet.sick = true;
    r.frames(2);
    r.lcd(&dir, "sick");
    // a stroll by day, then bedtime with the lights still on
    let mut r = Rig::seeded(Stage::Teen, "teen_g", 3);
    r.wander(5.0);
    r.core.hour_at = |_| 23.0;
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
    game_shot(&dir, "fire", Box::new(games::Fire::default()), 260, &[]);
    game_shot(&dir, "rock", Box::new(games::Rock::default()), 160, &[]);
    game_shot(&dir, "science", Box::new(games::Science::default()), 14, &[]);
    game_shot(&dir, "eats", VersusGame::Eats.make(), 30, &[Button::B]);
    game_shot(&dir, "flags", VersusGame::Flags.make(), 5, &[]);
    game_shot(&dir, "bump", VersusGame::Bump.make(), 45, &[]);
    println!("wrote art to {}", dir.display());
}
