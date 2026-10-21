# Bytepet

A virtual pet that lives on your desktop: a small see-through handheld you can drag
anywhere, on Windows, macOS and Linux. It plays like a mid-2000s "Connection-era"
handheld, modelled on the V4.5 feature set. You raise one pet at a time along three
skill paths, with school, careers, mail, marriage across generations, and linking
between devices over your LAN instead of infrared.

All characters, names and art are original. This is a fan-made homage to that style of
gameplay. It is not affiliated with Bandai or the Tamagotchi brand.

![Bytepet](docs/screenshot.png)

## Install

Download the build for your OS from the [latest release](https://github.com/Erdmann5150/Bytepet/releases/latest).

| OS | File | How |
|---|---|---|
| Windows 10/11 | `bytepet-…-windows-x86_64.zip` | Unzip and run `bytepet.exe`. If SmartScreen warns about an unrecognised app, choose *More info → Run anyway*. |
| macOS 11+ (Intel and Apple Silicon) | `bytepet-…-macos-universal.zip` | Unzip and move `Bytepet.app` to Applications. The first time, right-click it and choose *Open*. |
| Linux x86_64 | `bytepet-…-linux-x86_64.tar.gz` | Extract it and run `./install.sh`, which installs to `~/.local` with a menu entry. You can also run `./bytepet` directly. |

It's a single small program with nothing else to install. On Linux it needs glibc 2.35 or
newer (Ubuntu 22.04, Debian 12, Fedora 36 and later). The builds aren't code-signed, which is
why Windows and macOS warn the first time you run it.

Every download includes the [Player's Manual](https://github.com/Erdmann5150/Bytepet/releases/latest/download/Bytepet_Manual.pdf)
and a two-page [Quick Reference](https://github.com/Erdmann5150/Bytepet/releases/latest/download/Bytepet_Quick_Reference.pdf)
card as PDFs. These links always point to the newest versions.

### Uninstalling

- **Windows:** delete the program folder. To show notifications as "Bytepet", it also
  registers `HKCU\Software\Classes\AppUserModelId\Bytepet.Desktop` and keeps
  `bytepet-icon.png` in the save folder, so delete those too.
- **macOS:** delete `Bytepet.app`.
- **Linux:** run `./install.sh --remove`.

Your save stays where it is (see [Saves](#saves)). Delete it too if you want a clean slate.

## On your desktop

Bytepet opens as a floating, egg-shaped device in the bottom-right corner. Clicks on the
empty space around the egg pass through to whatever is behind it.

- **Drag** the shell to move it. Bytepet remembers where you left it.
- **Settings:** the everyday ones (speed, sound, shell) are in Town > SETUP, and **MORE** there
  opens all of them: size, shell, time speed, sound volume, always on top, notifications, launch at login, help, hide and quit. They open on the
  device's own screen, which switches to a finer dot grid to show them, so the rest of the device
  stays in sight while you change it. Work them with the device's buttons: **A** for the next row,
  **B** to change or open it, **C** to go back. With the mouse, **scroll** to pick a row,
  **click** to change or open it and **right-click** to go back. On the keyboard, up and down
  move, and left and right change a setting either way.
- **Make it yours:** the settings' SHELL page has 12 preset designs, and pages for the body
  (colour, pattern, pattern colour, shine), buttons (colour, shape, letters), antenna (shape,
  colour, key ring), screen (faceplate, rim, frame, corners, LCD colour) and icons (icon colour,
  the selected icon, the alert icon and the BYTEPET text). Each part changes on its own as you
  watch, and RANDOM changes every part at once.
- **Tray or menu-bar icon:** click it to show or hide the pet. Its menu has *Show / hide*,
  *Settings* (brings the pet forward with its settings open) and *Quit*. On Windows and macOS
  Bytepet has no taskbar or Dock button, so the tray icon is how you bring it back after hiding it.
- **Notifications** tell you when your pet calls, gets sick, hatches, grows up, gets mail or
  leaves, at most once every 2 minutes per kind.
- **Starting Bytepet again** brings the running pet forward instead of opening a second copy.
- `--windowed` runs it as an ordinary framed window instead.

### Linux notes

- **Wayland:** Bytepet runs through XWayland automatically, so staying on top, the
  remembered position and click-through all work. Only without XWayland (or with
  `BYTEPET_NATIVE_WAYLAND=1`) do those three features go away, because Wayland doesn't let
  apps do them.
- **GNOME** has no tray icon unless the *AppIndicator and KStatusNotifierItem Support*
  extension is on (Ubuntu turns it on by default). Without it, Bytepet shows in the taskbar
  instead. KDE, Xfce, Cinnamon and most other desktops show the tray icon out of the box.
- The see-through corners need a compositing desktop. Without one, they draw black.

## Controls

Keys work while Bytepet has focus, so click it first.

| Button | Keys | Meaning |
|---|---|---|
| A | `Z` `A` `J` `1` `←` | next / cycle |
| B | `X` `S` `K` `2` `↓` `Enter` `Space` | confirm (shows the clock on the main screen) |
| C | `C` `D` `L` `3` `→` `Esc` `Backspace` | back / cancel (clears the selected icon on the main screen) |

| Mouse | Does |
|---|---|
| Click A, B or C | that button |
| Click an icon | opens it, even from inside another menu |
| Click the screen | B |
| Scroll wheel | down for the next option (A), up for the previous one |
| Right-click | C: back, or clear the selected icon |
| Drag the shell or the screen | move Bytepet |

A+C together or `M` mutes. `F1` opens help, `F2` cycles time speed and `F3` cycles the preset
shell designs. When the game asks for a name or an IP address, type it on your keyboard.

## How it plays

Time runs in real time and follows your computer's clock, so the pet sleeps at night. Time
also keeps running while Bytepet is closed, up to 14 days. To pause instead, set
*Town → Setup → WHEN OFF* to PAUSE. Speeds of 10×, 60× and 600× are for trying things out; while
one is on, a badge above the screen shows it.

| Stage | Length | Notes |
|---|---|---|
| Egg | 1 minute | Name your pet first |
| Baby | 1 hour | Needs constant care; care mistakes here choose between two children |
| Child | 1 day | Games, mail, school and linking unlock |
| Teen | 2 days | The highest skill decides the path when it grows up |
| Adult | from day 3 | Jobs; the matchmaker from age 5 |
| Senior | age 8 if unmarried | Can only marry another senior, and that pairing produces a rare egg |

**Care.** Hunger and happiness are 4 hearts each. When either one empties, the pet calls with
the attention icon and a beep. A call left for 15 minutes counts as a care mistake. Clean up
poop, give medicine when the skull appears (some illnesses need two doses), and turn the
lights off once the pet falls asleep, or that counts as a care mistake too. A pet left sick or
starving for too long goes back to its home planet.

**Discipline.** Sometimes the pet calls when nothing is wrong, or refuses food: SCOLD it.
Sometimes it shows off with a little dance: PRAISE it. Getting these right fills the
training meter, and the best adult on each path needs a high one.

**Skills.** There are three: Humor, Gorgeous and Passion. CLIMB trains Humor, TUG trains
Gorgeous and APPLES trains Passion. MANHOLE earns Gotchi Points (GP). School letters let
you pick a teacher and sit a short quiz. Shop items and mail add more.

**Adults.** Each path has four adults, decided by care mistakes and discipline, plus a
secret adult for 200+ in that path's skill with zero care mistakes after the baby stage.
There are 23 characters in all.

**Jobs.** Job offers arrive by mail once a skill is high enough. Pass a short interview
quiz and WORK appears in the game menu, playable once every 2 hours. Office, Scientist,
Chef, Firefighter and Rock Star each have their own game.

**Town.** Mail, the shop (food, toys, a medkit), your items, PARTNER (the matchmaker), the
friends list, an album of past generations, and setup.

## Linking two Bytepets

Both computers must be on the same network.

1. On one, choose **LINK → HOST**.
2. On the other, choose **LINK → JOIN**, which finds hosts automatically, or **LINK → IP**
   and type the address shown on the host's screen.
3. Both screens show the same 4-digit code. If it matches, press **B on the host** to
   accept, or C to refuse. Nothing is exchanged before that.
4. The host picks one of four actions:
   - **VISIT:** the pets meet and their friendship goes up.
   - **GAME:** EATS, FLAGS or BUMP. Both sides play the same round and the scores are compared.
   - **GIFT:** send an item or 100 GP.
   - **MARRY:** needs SOULMATE friendship (10 link-ups) and two eligible adults, or two
     seniors. Both players get the next-generation egg.

Linking uses TCP port 41975 and UDP port 41976. Windows and macOS ask whether to allow
network access the first time you host: allow it on private networks. If 41975 is busy,
the host uses another port and shows it, for example `192.168.1.20:41980`; type the address
with the port. To try it on one computer, run two copies with different saves:
`bytepet --save a.json` and `bytepet --save b.json`.

## Saves

| OS | Location |
|---|---|
| Windows | `%APPDATA%\Bytepet\save.json` |
| macOS | `~/Library/Application Support/Bytepet/save.json` |
| Linux | `~/.local/share/bytepet/save.json` (or under `$XDG_DATA_HOME`) |

`bytepet --paths` prints the exact locations. The game saves every 20 seconds and on exit,
and a crash can't leave the file half-written. If the save is damaged, it's renamed to
`save.json.corrupt` and a new egg starts. A save from a newer version is played but never
overwritten.

## Modding

Characters, sprites, item prices, timings, jobs, mail and friendship levels all live in
three text files:

```bash
bytepet --export-content <save folder>/content
```

This writes `characters.toml`, `props.toml` and `game.toml` into a `content` folder next to
your save (`bytepet --paths` shows where). Edit them and restart Bytepet. To go back to the
built-in version of a file, delete it. Every sprite is a
16×16 grid of `#` and `.`, and the comments at the top of each file explain the format. If
a file has a mistake, Bytepet falls back to the built-in content and tells you what's wrong.

## Command line

```
bytepet [--save FILE] [--speed 1|10|60|600] [--new] [--windowed]
        [--export-content DIR] [--screenshot FILE] [--paths] [--version] [--help]
```

`--new` starts a fresh egg; the old save is overwritten at the next autosave.
`--screenshot` renders the device to a PNG and exits without opening a window.

## Building from source

You need Rust 1.80 or newer from [rustup.rs](https://rustup.rs). On Linux you also need the
ALSA headers: `sudo apt install libasound2-dev pkg-config` (Debian, Ubuntu) or
`sudo dnf install alsa-lib-devel` (Fedora).

```bash
cargo run --release                                                     # build and run
cargo test --no-default-features                                        # tests, no desktop libraries needed
cargo test --no-default-features --test robustness -- --ignored         # the long random-input fuzz
cargo fmt --check && cargo clippy --all-targets -- -D warnings          # the lint CI enforces
cargo run --example export_art --no-default-features -- art/            # every sprite and screen as PNG
```

To build the manual and quick reference PDFs, export the art as above, then run this
(needs Python 3.11+ and `pip install reportlab`):

```bash
python3 tools/build_manual.py art/ dist/
```

CI runs the tests on all three platforms, lints, and builds the packages and the two PDFs.
Pushing a tag such as `v2.1.0` publishes a GitHub release with all of them attached.

```
content/          characters, props and game rules (TOML; embedded, overridable)
src/
  sim.rs          the pet simulation: timers, care, evolution, jobs, mail, marriage
  content.rs      loads the content files and any mods
  screens/        every LCD screen as a screen stack
  games/          the mini-games
  link.rs net.rs  the LINK menu and LAN networking
  core.rs         ties sim, screens, input and saving together, no OS dependencies
  render.rs       draws the device; layout.rs has its geometry
  look.rs         the customisable look: colours, shapes, patterns and the preset shells
  lcd.rs font.rs  the dot-matrix screen and its font
  save.rs         versioned save files
  instance.rs     one running copy per save file
  overlay.rs      the settings and help pages, shown on the device's screen
  main.rs         command line, then hands off to desktop/
  desktop/        window, tray, notifications, sound, autostart, click-through
tests/            end-to-end flows, two-device link tests, fuzzing
examples/         export_art.rs: sprites and screens as PNG, for the manual
tools/            build_manual.py: the manual and quick reference PDFs, from the content and art
packaging/        macOS Info.plist, Linux .desktop file and installer
assets/           app icons
```
