// No console window on Windows.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod desktop;

use std::path::PathBuf;

use bytepet::content::Content;
use bytepet::core::Core;
use bytepet::render::Renderer;
use bytepet::save;

const HELP: &str = "Bytepet - a virtual pet that lives on your desktop

Usage: bytepet [options]

  --save FILE           use a different save file
  --speed 1|10|60|600   time speed (pet time per real second)
  --new                 ignore the existing save and start a new egg
  --windowed            a normal window instead of the floating widget
  --export-content DIR  write the built-in content files there, to mod
  --screenshot FILE     render the device to a PNG and exit
  --paths               show where the save and mod files live
  --version, --help
";

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut save_arg, mut speed, mut fresh, mut windowed, mut shot, mut export, mut paths) =
        (None, None, false, false, None, None, false);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--save" => save_arg = args.next(),
            "--speed" => speed = args.next().and_then(|s| s.parse::<u32>().ok()),
            "--new" => fresh = true,
            "--windowed" => windowed = true,
            "--screenshot" => shot = args.next(),
            "--export-content" => export = args.next(),
            "--paths" => paths = true,
            "--version" => return println!("bytepet {}", env!("CARGO_PKG_VERSION")),
            "--help" | "-h" => return print!("{HELP}"),
            other => {
                eprintln!("unknown option {other}\n\n{HELP}");
                std::process::exit(2);
            }
        }
    }

    let mod_dir = save::data_dir().join("content");
    let save_path = save_arg.as_ref().map(PathBuf::from).unwrap_or_else(save::default_save_path);
    if paths {
        println!("save file:  {}\nmod folder: {}", save_path.display(), mod_dir.display());
        return;
    }
    if let Some(dir) = export {
        let dir = PathBuf::from(dir);
        let files = [
            ("characters.toml", include_str!("../content/characters.toml")),
            ("props.toml", include_str!("../content/props.toml")),
            ("game.toml", include_str!("../content/game.toml")),
        ];
        for (name, text) in files {
            if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(name), text)) {
                eprintln!("could not write {name}: {e}");
                std::process::exit(1);
            }
        }
        return println!("wrote the content files to {}", dir.display());
    }

    let mut mod_note = None;
    let content = match Content::load(Some(&mod_dir)) {
        Ok(c) => c,
        Err(e) => {
            // TOML errors are multi-line with a source excerpt; keep the location and the reason
            let mut lines = e.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('|'));
            let first = lines.next().unwrap_or_default();
            let why = lines.next_back().map(|l| format!(": {l}")).unwrap_or_default();
            mod_note =
                Some(format!("Ignoring the mods in {} ({first}{why}). Using the built-in content.", mod_dir.display()));
            Content::builtin()
        }
    };
    // one copy per save file: a second launch brings the running pet forward instead
    let mut instance = None;
    if shot.is_none() {
        match bytepet::instance::claim(&save_path) {
            bytepet::instance::Claim::First(server) => instance = Some(server),
            bytepet::instance::Claim::Running => {
                eprintln!("bytepet: already running with {}; showing it", save_path.display());
                return;
            }
            bytepet::instance::Claim::Unknown => {}
        }
    }
    let mut core = Core::open(content, save_path, fresh);
    core.startup_notes.extend(mod_note);
    for n in &core.startup_notes {
        eprintln!("bytepet: {n}");
    }
    if let Some(s) = speed.filter(|s| [1, 10, 60, 600].contains(s)) {
        core.game.settings.speed = s;
    }

    if let Some(file) = shot {
        core.frame(0.1);
        core.shade = core.lcd.buf.map(|v| v as f32);
        let pm = Renderer::new().render(&core, core.game.settings.ui.scale, 1.0, None, bytepet::core::now_unix());
        if let Err(e) = pm.save_png(&file) {
            eprintln!("could not write {file}: {e}");
            std::process::exit(1);
        }
        return;
    }

    let opts = desktop::Options { windowed, save_arg, instance };
    if let Err(e) = desktop::run(core, opts) {
        eprintln!("bytepet: {e}");
        std::process::exit(1);
    }
}
