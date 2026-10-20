//! The tray / menu-bar icon: click to show or hide the pet; its menu holds the settings, Help and Quit.
//! Windows and macOS use tray-icon; Linux uses the StatusNotifierItem D-Bus protocol (ksni),
//! which KDE, Xfce, Cinnamon and GNOME-with-AppIndicator understand.

use winit::event_loop::EventLoopProxy;

use bytepet::layout::SHELLS;
use bytepet::overlay::{size_name, Choice, SIZES};
use bytepet::screens::town::SPEEDS;
use bytepet::sim::{Settings, VOLUMES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCmd {
    Toggle,
    /// Set a setting to a value.
    Pick(Choice),
    /// Turn an on/off setting over.
    Flip(Flag),
    Help,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    OnTop,
    Notify,
    Login,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserEvent {
    Tray(TrayCmd),
    /// Bytepet was launched again with the same save.
    Show,
}

/// What the menu's checkmarks show.
#[derive(Debug, Clone, PartialEq)]
pub struct TrayState {
    size: usize,
    shell: u32,
    speed: u32,
    sound: Option<u32>,
    on_top: bool,
    notify: bool,
    login: bool,
}

impl TrayState {
    pub fn new(s: &Settings, login: bool) -> TrayState {
        TrayState {
            size: SIZES.iter().position(|x| x.0 == size_name(s.ui.scale)).unwrap_or(1),
            shell: s.shell,
            speed: s.speed,
            sound: s.sound(),
            on_top: s.ui.on_top,
            notify: s.ui.notify,
            login,
        }
    }

    fn picked(&self, c: Choice) -> bool {
        match c {
            Choice::Size(i) => i == self.size,
            Choice::Shell(i) => i == self.shell,
            Choice::Speed(x) => x == self.speed,
            Choice::Sound(v) => v == self.sound,
            _ => false,
        }
    }

    fn flag(&self, f: Flag) -> bool {
        match f {
            Flag::OnTop => self.on_top,
            Flag::Notify => self.notify,
            Flag::Login => self.login,
        }
    }
}

/// The settings submenus: a title, then a label and choice per option.
fn submenus() -> Vec<(&'static str, Vec<(String, Choice)>)> {
    let mut sound = vec![("Off".to_string(), Choice::Sound(None))];
    sound.extend(VOLUMES.iter().map(|&v| (format!("{v}%"), Choice::Sound(Some(v)))));
    vec![
        ("Size", SIZES.iter().enumerate().map(|(i, s)| (title_case(s.0), Choice::Size(i))).collect()),
        ("Shell", SHELLS.iter().enumerate().map(|(i, s)| (title_case(s.name), Choice::Shell(i as u32))).collect()),
        ("Speed", SPEEDS.iter().map(|&x| (format!("{x}\u{d7}"), Choice::Speed(x))).collect()),
        ("Sound", sound),
    ]
}

const FLAGS: [(&str, Flag); 3] =
    [("Always on top", Flag::OnTop), ("Notifications", Flag::Notify), ("Launch at login", Flag::Login)];

/// "BERRY" -> "Berry": the LCD's capitals read as shouting in a system menu.
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_string() + &c.as_str().to_lowercase()).unwrap_or_default()
}

/// A 32x32 RGBA icon: the pet on a pink disc.
pub fn icon_rgba(c: &bytepet::content::Content) -> (Vec<u8>, u32, u32) {
    let size = 32u32;
    let spr = c.sprite("child_a", bytepet::content::Pose::Happy);
    let mut px = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let i = ((y * size + x) * 4) as usize;
            if dx * dx + dy * dy <= 15.5 * 15.5 {
                px[i..i + 4].copy_from_slice(&[236, 72, 153, 255]);
            }
            let (sx, sy) = (x as i32 / 2, y as i32 / 2);
            let idx = (sy * spr.w + sx) as usize;
            if sx < spr.w && sy < spr.h {
                if spr.pix[idx] {
                    px[i..i + 4].copy_from_slice(&[30, 30, 30, 255]);
                } else if spr.mask[idx] {
                    px[i..i + 4].copy_from_slice(&[248, 246, 238, 255]);
                }
            }
        }
    }
    (px, size, size)
}

// ====================================================================== Windows / macOS
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub struct Tray {
    _icon: tray_icon::TrayIcon,
    picks: Vec<(tray_icon::menu::CheckMenuItem, Choice)>,
    flags: Vec<(tray_icon::menu::CheckMenuItem, Flag)>,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn menu_id(cmd: TrayCmd) -> String {
    format!("{cmd:?}")
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
impl Tray {
    /// Must be called on the main thread after the event loop has started.
    pub fn new(proxy: EventLoopProxy<UserEvent>, icon: (Vec<u8>, u32, u32), state: &TrayState) -> Option<Tray> {
        use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
        use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

        let menu = Menu::new();
        menu.append(&MenuItem::with_id(menu_id(TrayCmd::Toggle), "Show / hide Bytepet", true, None)).ok()?;
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        let mut picks = Vec::new();
        for (title, options) in submenus() {
            let sub = Submenu::new(title, true);
            for (label, c) in options {
                let item = CheckMenuItem::with_id(menu_id(TrayCmd::Pick(c)), label, true, state.picked(c), None);
                sub.append(&item).ok()?;
                picks.push((item, c));
            }
            menu.append(&sub).ok()?;
        }
        let mut flags = Vec::new();
        for (label, f) in FLAGS {
            let item = CheckMenuItem::with_id(menu_id(TrayCmd::Flip(f)), label, true, state.flag(f), None);
            menu.append(&item).ok()?;
            flags.push((item, f));
        }
        menu.append(&MenuItem::with_id(menu_id(TrayCmd::Help), "Help", true, None)).ok()?;
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        menu.append(&MenuItem::with_id(menu_id(TrayCmd::Quit), "Quit Bytepet", true, None)).ok()?;

        let mut cmds = vec![TrayCmd::Toggle, TrayCmd::Help, TrayCmd::Quit];
        cmds.extend(picks.iter().map(|&(_, c)| TrayCmd::Pick(c)));
        cmds.extend(flags.iter().map(|&(_, f)| TrayCmd::Flip(f)));
        let p1 = proxy.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Some(&cmd) = cmds.iter().find(|&&c| e.id == menu_id(c).as_str()) {
                let _ = p1.send_event(UserEvent::Tray(cmd));
            }
        }));
        let p2 = proxy;
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                let _ = p2.send_event(UserEvent::Tray(TrayCmd::Toggle));
            }
        }));

        let (rgba, w, h) = icon;
        let icon = Icon::from_rgba(rgba, w, h).ok()?;
        let tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("Bytepet")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .ok()?;
        Some(Tray { _icon: tray, picks, flags })
    }

    /// Show the current settings. Also needed after every menu click: a clicked check item
    /// turns itself over, even one that should stay as it is.
    pub fn sync(&self, state: &TrayState) {
        for (item, c) in &self.picks {
            item.set_checked(state.picked(*c));
        }
        for (item, f) in &self.flags {
            item.set_checked(state.flag(*f));
        }
    }
}

// ====================================================================== Linux
#[cfg(target_os = "linux")]
pub struct Tray {
    handle: ksni::blocking::Handle<SniTray>,
}

#[cfg(target_os = "linux")]
pub struct SniTray {
    proxy: EventLoopProxy<UserEvent>,
    icon: (Vec<u8>, u32, u32),
    state: TrayState,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for SniTray {
    fn id(&self) -> String {
        "bytepet".into()
    }
    fn title(&self) -> String {
        "Bytepet".into()
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let (rgba, w, h) = &self.icon;
        // SNI wants ARGB32 in network byte order
        let data = rgba.chunks(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
        vec![ksni::Icon { width: *w as i32, height: *h as i32, data }]
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.proxy.send_event(UserEvent::Tray(TrayCmd::Toggle));
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, RadioGroup, RadioItem, StandardItem, SubMenu};
        let send = |cmd: TrayCmd| {
            move |t: &mut SniTray| {
                let _ = t.proxy.send_event(UserEvent::Tray(cmd));
            }
        };
        let item = |label: &str, cmd: TrayCmd| -> ksni::MenuItem<SniTray> {
            StandardItem { label: label.into(), activate: Box::new(send(cmd)), ..Default::default() }.into()
        };
        let mut menu = vec![item("Show / hide Bytepet", TrayCmd::Toggle), ksni::MenuItem::Separator];
        for (title, options) in submenus() {
            // nothing is ticked if the save holds a value the menu doesn't offer
            let selected = options.iter().position(|&(_, c)| self.state.picked(c)).unwrap_or(usize::MAX);
            let choices: Vec<Choice> = options.iter().map(|&(_, c)| c).collect();
            let group = RadioGroup {
                selected,
                select: Box::new(move |t: &mut SniTray, i: usize| {
                    if let Some(&c) = choices.get(i) {
                        let _ = t.proxy.send_event(UserEvent::Tray(TrayCmd::Pick(c)));
                    }
                }),
                options: options.into_iter().map(|(label, _)| RadioItem { label, ..Default::default() }).collect(),
            };
            menu.push(SubMenu { label: title.into(), submenu: vec![group.into()], ..Default::default() }.into());
        }
        for (label, f) in FLAGS {
            let check = CheckmarkItem {
                label: label.into(),
                checked: self.state.flag(f),
                activate: Box::new(send(TrayCmd::Flip(f))),
                ..Default::default()
            };
            menu.push(check.into());
        }
        menu.push(item("Help", TrayCmd::Help));
        menu.push(ksni::MenuItem::Separator);
        menu.push(item("Quit Bytepet", TrayCmd::Quit));
        menu
    }
}

#[cfg(target_os = "linux")]
impl Tray {
    pub fn new(proxy: EventLoopProxy<UserEvent>, icon: (Vec<u8>, u32, u32), state: &TrayState) -> Option<Tray> {
        use ksni::blocking::TrayMethods;
        // If no tray host is running (stock GNOME), the icon simply never appears; the window
        // stays in the taskbar, so nothing is lost.
        let tray = SniTray { proxy, icon, state: state.clone() };
        let handle = tray.assume_sni_available(true).spawn().ok()?;
        Some(Tray { handle })
    }

    /// Show the current settings.
    pub fn sync(&self, state: &TrayState) {
        self.handle.update(|t| t.state = state.clone());
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub struct Tray;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl Tray {
    pub fn new(_proxy: EventLoopProxy<UserEvent>, _icon: (Vec<u8>, u32, u32), _state: &TrayState) -> Option<Tray> {
        None
    }
    pub fn sync(&self, _state: &TrayState) {}
}
