//! The desktop shell: a frameless, see-through, always-on-top window that shows the device,
//! plus the tray icon, notifications, sound and launch-at-login.

mod audio;
mod autostart;
mod gfx;
mod notify;
mod shape;
mod tray;
#[cfg(target_os = "windows")]
mod win_frame;
#[cfg(target_os = "windows")]
mod win_identity;

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tiny_skia::{Pixmap, PixmapPaint, Transform};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Icon, Window, WindowId, WindowLevel};

use bytepet::core::{now_unix, Core};
use bytepet::layout::{button_at, icon_at, on_device, on_lcd, VIEW};
use bytepet::overlay::{self, DesktopCmd, Env, Page};
use bytepet::render::{window_size, Renderer};
use bytepet::screens::Key as ScreenKey;
use bytepet::Button;

use gfx::Gfx;
use tray::{Tray, TrayCmd, UserEvent};

static TERMINATE: AtomicBool = AtomicBool::new(false);

pub struct Options {
    /// A normal window with a title bar instead of the floating widget.
    pub windowed: bool,
    /// Passed to launch-at-login when a non-default save file is used.
    pub save_arg: Option<String>,
    /// Held while running so a second launch with the same save shows this one instead.
    pub instance: Option<bytepet::instance::Server>,
}

const FRAME: Duration = Duration::from_millis(33);
const FRAME_HIDDEN: Duration = Duration::from_millis(250);
const HIDE_TO_TRAY: bool = cfg!(any(target_os = "windows", target_os = "macos"));

pub fn run(core: Core, mut opts: Options) -> Result<(), Box<dyn Error>> {
    install_signal_handlers();
    #[allow(unused_mut)] // only mutated on Linux and macOS
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    {
        // XWayland lets us stay on top and remember our position; native Wayland can't.
        use winit::platform::x11::EventLoopBuilderExtX11;
        if std::env::var_os("DISPLAY").is_some() && std::env::var_os("BYTEPET_NATIVE_WAYLAND").is_none() {
            builder.with_x11();
        }
    }
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        if !opts.windowed {
            builder.with_activation_policy(ActivationPolicy::Accessory); // no Dock icon
        }
    }
    let event_loop = builder.build()?;
    let proxy = event_loop.create_proxy();
    if let Some(server) = opts.instance.take() {
        let p = proxy.clone();
        bytepet::instance::serve(server, move || {
            let _ = p.send_event(UserEvent::Show);
        });
    }
    let mut app = App::new(core, opts, proxy);
    // after App::new, which sets the macOS notification identity
    if !app.core.startup_notes.is_empty() {
        // a damaged save or broken mod would otherwise only be reported on a console nobody sees
        notify::plain("Bytepet", &app.core.startup_notes.join("\n"));
    }
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    core: Core,
    opts: Options,
    proxy: EventLoopProxy<UserEvent>,
    renderer: Renderer,
    gfx: Option<Gfx>,
    tray: Option<Tray>,
    tried_tray: bool,
    notifier: notify::Notifier,
    audio: audio::Audio,
    login: bool,
    cursor: (f64, f64),
    pressed: Option<Button>,
    /// Where a left press on the LCD (or, in the settings, anywhere off the buttons) started: a
    /// click there is B, a drag moves the window.
    lcd_press: Option<(f64, f64)>,
    /// Windows: where in the window the drag moving it began (see `start_drag`).
    drag: Option<(f64, f64)>,
    /// Unused scroll-wheel movement, in notches.
    wheel: f64,
    /// The outline the window's click-through shape currently follows.
    shaped: Option<shape::Outline>,
    last_tick: Instant,
    visible: bool,
    quit: bool,
}

impl App {
    fn new(core: Core, opts: Options, proxy: EventLoopProxy<UserEvent>) -> App {
        App {
            core,
            opts,
            proxy,
            renderer: Renderer::new(),
            gfx: None,
            tray: None,
            tried_tray: false,
            notifier: notify::Notifier::new(),
            audio: audio::Audio::new(),
            login: autostart::is_enabled(),
            cursor: (0.0, 0.0),
            pressed: None,
            lcd_press: None,
            drag: None,
            wheel: 0.0,
            shaped: None,
            last_tick: Instant::now(),
            visible: true,
            quit: false,
        }
    }

    fn env(&self) -> Env {
        let hide_label = if HIDE_TO_TRAY && self.tray.is_some() { "HIDE TO TRAY" } else { "MINIMIZE" };
        Env { login: self.login, hide_label }
    }

    fn window(&self) -> Option<&Window> {
        self.gfx.as_ref().map(|g| &g.window)
    }

    /// Physical pixels per design unit.
    fn px_scale(&self) -> f32 {
        self.window().map(|w| w.inner_size().width as f32 / VIEW.2).unwrap_or(1.0)
    }

    fn to_design(&self, (x, y): (f64, f64)) -> (f32, f32) {
        let s = self.px_scale();
        (x as f32 / s + VIEW.0, y as f32 / s + VIEW.1)
    }

    fn create_window(&mut self, el: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let ui = self.core.game.settings.ui.clone();
        let (w, h) = window_size(ui.scale);
        let (rgba, iw, ih) = tray::icon_rgba(&self.core.content);
        let mut a = Window::default_attributes()
            .with_title("Bytepet")
            .with_inner_size(LogicalSize::new(w as f64, h as f64))
            .with_resizable(false)
            .with_window_icon(Icon::from_rgba(rgba, iw, ih).ok());
        if !self.opts.windowed {
            a = a.with_decorations(false).with_transparent(true).with_window_level(if ui.on_top {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            });
            #[cfg(target_os = "windows")]
            {
                use winit::platform::windows::WindowAttributesExtWindows;
                if self.tray.is_some() {
                    a = a.with_skip_taskbar(true);
                }
            }
        }
        // restore the saved spot if it's still on a screen; otherwise bottom-right of the main one
        let on_screen = |x: i32, y: i32| {
            el.available_monitors().any(|m| {
                let (p, s) = (m.position(), m.size());
                x + 40 >= p.x && y + 40 >= p.y && x + 40 < p.x + s.width as i32 && y + 40 < p.y + s.height as i32
            })
        };
        match (ui.x, ui.y) {
            (Some(x), Some(y)) if on_screen(x, y) => a = a.with_position(PhysicalPosition::new(x, y)),
            _ => {
                if let Some(m) = el.primary_monitor().or_else(|| el.available_monitors().next()) {
                    let dpi = m.scale_factor();
                    let (p, s) = (m.position(), m.size());
                    let x = p.x + s.width as i32 - ((w as f64 + 40.0) * dpi) as i32;
                    let y = p.y + s.height as i32 - ((h as f64 + 80.0) * dpi) as i32;
                    a = a.with_position(PhysicalPosition::new(x, y));
                }
            }
        }
        let gfx = Gfx::new(el, a)?;
        if !self.opts.windowed {
            #[cfg(target_os = "windows")]
            win_frame::install(&gfx.window);
            let outline = shape::Outline::new(&self.core.game.settings.look);
            shape::apply(&gfx.window, gfx.window.inner_size().width as f32 / VIEW.2, &outline);
            self.shaped = Some(outline);
        }
        if !gfx.transparent && !self.opts.windowed {
            eprintln!("bytepet: this display can't do transparent windows; the corners will be black");
        }
        gfx.window.request_redraw();
        self.gfx = Some(gfx);
        Ok(())
    }

    /// The window's click-through shape follows the antenna and key ring, which the look can
    /// change.
    fn update_shape(&mut self) {
        if self.opts.windowed {
            return;
        }
        let Some(win) = self.window() else { return };
        let size = win.inner_size();
        let want = shape::Outline::new(&self.core.game.settings.look);
        if Some(want) != self.shaped {
            shape::apply(win, size.width as f32 / VIEW.2, &want);
            self.shaped = Some(want);
        }
    }

    fn redraw(&mut self) {
        self.update_shape();
        let wall = now_unix();
        let Some(win) = self.window() else { return };
        let dpi = win.scale_factor() as f32;
        let s = self.px_scale();
        let mut pm = self.renderer.render(&self.core, s, dpi, wall);
        if self.opts.windowed {
            // no see-through in a normal window: put the device on a dark backdrop
            if let Some(mut bg) = Pixmap::new(pm.width(), pm.height()) {
                bg.fill(tiny_skia::Color::from_rgba8(28, 30, 38, 255));
                bg.draw_pixmap(0, 0, pm.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
                pm = bg;
            }
        }
        if let Some(g) = self.gfx.as_mut() {
            g.present(pm.data(), pm.width(), pm.height());
        }
    }

    fn show(&mut self) {
        if let Some(w) = self.window() {
            w.set_visible(true);
            w.set_minimized(false);
            w.focus_window();
            w.request_redraw();
        }
        self.visible = true;
    }

    fn hide(&mut self) {
        if HIDE_TO_TRAY && self.tray.is_some() {
            if let Some(w) = self.window() {
                w.set_visible(false);
            }
            self.visible = false;
        } else if let Some(w) = self.window() {
            w.set_minimized(true);
        }
    }

    fn desktop_cmd(&mut self, cmd: DesktopCmd) {
        match cmd {
            DesktopCmd::SetScale(s) => {
                let (w, h) = window_size(s);
                if let Some(win) = self.window() {
                    let _ = win.request_inner_size(LogicalSize::new(w as f64, h as f64));
                }
            }
            DesktopCmd::SetOnTop(on) => {
                if let Some(win) = self.window() {
                    win.set_window_level(if on { WindowLevel::AlwaysOnTop } else { WindowLevel::Normal });
                }
            }
            DesktopCmd::SetLogin(on) => {
                autostart::set_enabled(on, self.opts.save_arg.as_deref());
                self.login = autostart::is_enabled();
            }
            DesktopCmd::Hide => self.hide(),
            DesktopCmd::Quit => self.quit = true,
        }
    }

    /// Carry out what the settings asked of the desktop: a command from a click or key here,
    /// and any left by button presses (see `Core::press`).
    fn run_cmds(&mut self, cmd: Option<DesktopCmd>) {
        for c in cmd.into_iter().chain(self.core.overlay.take_cmds()) {
            self.desktop_cmd(c);
        }
    }

    /// An arrow key in the settings: acts on every press, held keys repeating.
    fn menu_key(&mut self, down: bool, f: impl FnOnce(&mut Core) -> Option<DesktopCmd>) {
        if down {
            let cmd = f(&mut self.core);
            self.run_cmds(cmd);
        }
    }

    fn tray_cmd(&mut self, cmd: TrayCmd) {
        match cmd {
            TrayCmd::Toggle => {
                if self.visible && !self.window().map(|w| w.is_minimized().unwrap_or(false)).unwrap_or(false) {
                    self.hide();
                } else {
                    self.show();
                }
            }
            TrayCmd::Settings => {
                self.show();
                if !self.core.overlay.has_rows() {
                    self.core.overlay.open();
                }
            }
            TrayCmd::Quit => self.quit = true,
        }
        if let Some(w) = self.window() {
            w.request_redraw();
        }
    }

    /// Move the window with the mouse, from a press at `from` (window pixels). On Windows we
    /// move it ourselves: `drag_window` runs the system's move loop, which holds up the event
    /// loop, and so the animation, until the button is released.
    fn start_drag(&mut self, from: (f64, f64)) {
        if cfg!(target_os = "windows") {
            self.drag = Some(from);
            self.follow_drag();
        } else if let Some(w) = self.window() {
            let _ = w.drag_window();
        }
    }

    /// Keep the point where the drag began under the cursor. The window has the mouse captured
    /// while the button is down, so this keeps up even when the cursor outruns it.
    fn follow_drag(&self) {
        let (Some((sx, sy)), Some(w)) = (self.drag, self.window()) else { return };
        let Ok(p) = w.outer_position() else { return };
        let (dx, dy) = ((self.cursor.0 - sx).round() as i32, (self.cursor.1 - sy).round() as i32);
        if dx != 0 || dy != 0 {
            w.set_outer_position(PhysicalPosition::new(p.x + dx, p.y + dy));
        }
    }

    /// Mouse: click A/B/C or an icon; click the screen for B, scroll for A; right-click is C
    /// (see `Core::right_click`). While the settings are open the wheel picks a row, a click anywhere
    /// but the buttons chooses it (B) and right-click goes back; the buttons work them too (see
    /// `Core::press`).
    fn mouse(&mut self, state: ElementState, button: MouseButton) {
        let down = state == ElementState::Pressed;
        match (button, down) {
            (MouseButton::Right, true) => self.core.right_click(),
            (MouseButton::Left, true) => {
                let (x, y) = self.to_design(self.cursor);
                if let Some(b) = button_at(x, y) {
                    self.pressed = Some(b);
                    self.core.press(b);
                    self.run_cmds(None);
                } else if self.core.overlay.is_open() {
                    self.lcd_press = Some(self.cursor);
                } else if let Some(i) = icon_at(x, y) {
                    self.core.click_icon(i);
                } else if on_lcd(x, y) {
                    self.lcd_press = Some(self.cursor);
                } else if !self.opts.windowed && on_device(x, y, &self.core.game.settings.look) {
                    self.start_drag(self.cursor);
                }
            }
            (MouseButton::Left, false) => {
                self.drag = None;
                if let Some(b) = self.pressed.take() {
                    self.core.release(b);
                }
                if self.lcd_press.take().is_some() {
                    self.core.tap(Button::B);
                    self.run_cmds(None);
                }
            }
            _ => {}
        }
    }

    fn wheel(&mut self, delta: MouseScrollDelta) {
        // positive = scrolled up, one unit per notch
        self.wheel += match delta {
            MouseScrollDelta::LineDelta(_, y) => y as f64,
            MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
        };
        while self.wheel.abs() >= 1.0 {
            let up = self.wheel > 0.0;
            self.wheel -= if up { 1.0 } else { -1.0 };
            if self.core.overlay.page == Page::Help {
                self.core.overlay.next_help();
            } else if self.core.overlay.is_open() {
                overlay::scroll(&mut self.core, if up { -1 } else { 1 });
            } else {
                self.core.scroll(up);
            }
        }
    }

    fn key(&mut self, ev: &KeyEvent) {
        let down = ev.state == ElementState::Pressed;
        let ch = match &ev.logical_key {
            Key::Character(s) => s.to_lowercase(),
            _ => String::new(),
        };
        let named = match &ev.logical_key {
            Key::Named(n) => Some(*n),
            _ => None,
        };

        // the settings take the keyboard while they're open: the arrows move between rows and
        // change them, and the keys for A, B and C press the device's buttons, which work them
        if self.core.overlay.is_open() {
            let b = match (named, ch.as_str()) {
                (Some(NamedKey::ArrowDown), _) => return self.menu_key(down, |c| scroll_menu(c, 1)),
                (Some(NamedKey::ArrowUp), _) => return self.menu_key(down, |c| scroll_menu(c, -1)),
                (Some(NamedKey::ArrowLeft), _) => return self.menu_key(down, |c| overlay::adjust(c, -1)),
                (Some(NamedKey::ArrowRight), _) => return self.menu_key(down, |c| overlay::adjust(c, 1)),
                (_, "z" | "a" | "1" | "j") => Button::A,
                (Some(NamedKey::Enter | NamedKey::Space), _) | (_, "x" | "s" | "2" | "k") => Button::B,
                (Some(NamedKey::Escape | NamedKey::Backspace), _) | (_, "c" | "d" | "3" | "l") => Button::C,
                _ => return,
            };
            if down && !ev.repeat {
                self.core.press(b);
                self.run_cmds(None);
            } else if !down {
                self.core.release(b);
            }
            return;
        }

        if down && !ev.repeat {
            match named {
                Some(NamedKey::F1) => {
                    self.core.overlay.open_help();
                    return;
                }
                Some(NamedKey::F2) => return self.core.cycle_speed(),
                Some(NamedKey::F3) => return self.core.next_shell(),
                _ => {}
            }
        }

        if self.core.wants_text() {
            let b = match named {
                Some(NamedKey::ArrowLeft) => Some(Button::A),
                Some(NamedKey::ArrowDown) => Some(Button::B),
                Some(NamedKey::ArrowRight) | Some(NamedKey::Escape) => Some(Button::C),
                _ => None,
            };
            if let Some(b) = b {
                if down && !ev.repeat {
                    self.core.press(b);
                } else if !down {
                    self.core.release(b);
                }
            } else if down {
                match named {
                    Some(NamedKey::Backspace) => self.core.key(ScreenKey::Backspace),
                    Some(NamedKey::Enter) => self.core.key(ScreenKey::Enter),
                    _ => {
                        if let Some(t) = &ev.text {
                            let t: String = t.chars().filter(|c| !c.is_control()).collect();
                            if !t.is_empty() {
                                self.core.text(&t);
                            }
                        }
                    }
                }
            }
            return;
        }

        if down && !ev.repeat && ch == "m" {
            return self.core.toggle_mute();
        }
        let b = match (named, ch.as_str()) {
            (Some(NamedKey::ArrowLeft), _) | (_, "z" | "a" | "1" | "j") => Some(Button::A),
            (Some(NamedKey::ArrowDown | NamedKey::Enter | NamedKey::Space), _) | (_, "x" | "s" | "2" | "k") => {
                Some(Button::B)
            }
            (Some(NamedKey::ArrowRight | NamedKey::Escape | NamedKey::Backspace), _) | (_, "c" | "d" | "3" | "l") => {
                Some(Button::C)
            }
            _ => None,
        };
        if let Some(b) = b {
            if down && !ev.repeat {
                self.core.press(b);
            } else if !down {
                self.core.release(b);
            }
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        self.core.frame(dt);
        for t in self.core.take_sfx() {
            if let Some(volume) = self.core.game.settings.sound() {
                self.audio.play(t, volume);
            }
        }
        self.audio.idle();
        for e in self.core.take_notices() {
            self.notifier.event(&e, &self.core.game, &self.core.content);
        }
        self.core.overlay.env = self.env();
        if self.core.ui.request.take().is_some() {
            self.show();
            self.core.overlay.open();
        }
        if TERMINATE.load(Ordering::SeqCst) {
            self.quit = true;
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if !self.tried_tray && !self.opts.windowed {
            self.tried_tray = true;
            self.tray = Tray::new(self.proxy.clone(), tray::icon_rgba(&self.core.content));
            self.core.overlay.env = self.env();
        }
        if self.gfx.is_none() {
            if let Err(e) = self.create_window(el) {
                eprintln!("bytepet: could not open the window: {e}");
                self.core.save();
                el.exit();
            }
        }
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, ev: UserEvent) {
        match ev {
            UserEvent::Tray(cmd) => self.tray_cmd(cmd),
            UserEvent::Show => self.show(),
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => self.quit = true,
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &self.gfx {
                    g.resize(size.width, size.height);
                    if !self.opts.windowed {
                        let outline = shape::Outline::new(&self.core.game.settings.look);
                        shape::apply(&g.window, size.width as f32 / VIEW.2, &outline);
                        self.shaped = Some(outline);
                    }
                    g.window.request_redraw();
                }
            }
            WindowEvent::Moved(p) => {
                self.core.game.settings.ui.x = Some(p.x);
                self.core.game.settings.ui.y = Some(p.y);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.follow_drag();
                // a press on the LCD that turns into a drag moves the window instead of pressing B
                if let Some((sx, sy)) = self.lcd_press {
                    let (dx, dy) = (position.x - sx, position.y - sy);
                    if !self.opts.windowed && dx * dx + dy * dy > 36.0 {
                        self.lcd_press = None;
                        self.start_drag((sx, sy));
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => self.wheel(delta),
            WindowEvent::MouseInput { state, button, .. } => self.mouse(state, button),
            WindowEvent::KeyboardInput { event, .. } => self.key(&event),
            WindowEvent::Focused(false) => {
                // don't leave a button stuck down if focus leaves mid-press
                for b in Button::ALL {
                    self.core.release(b);
                }
                self.pressed = None;
                self.lcd_press = None;
                self.drag = None;
            }
            _ => {}
        }
        if self.quit {
            self.core.save();
            el.exit();
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let minimized = self.window().and_then(|w| w.is_minimized()).unwrap_or(false);
        let interval = if self.visible && !minimized { FRAME } else { FRAME_HIDDEN };
        if self.last_tick.elapsed() >= interval {
            self.tick();
            if self.visible && !minimized {
                if let Some(w) = self.window() {
                    w.request_redraw();
                }
            }
        }
        if self.quit {
            self.core.save();
            el.exit();
            return;
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.last_tick + interval));
    }

    fn exiting(&mut self, _el: &ActiveEventLoop) {
        self.core.save();
    }
}

#[cfg(unix)]
fn install_signal_handlers() {
    extern "C" fn on_signal(_: libc::c_int) {
        TERMINATE.store(true, Ordering::SeqCst);
    }
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGHUP, on_signal as *const () as libc::sighandler_t);
    }
}

#[cfg(not(unix))]
fn install_signal_handlers() {}

/// Up or down a row of the settings, for the arrow keys.
fn scroll_menu(core: &mut Core, steps: i32) -> Option<DesktopCmd> {
    overlay::scroll(core, steps);
    None
}
