//! The tray / menu-bar icon: click to show or hide the pet; its menu holds the settings, Help and Quit.
//! Windows and macOS use tray-icon; Linux uses the StatusNotifierItem D-Bus protocol (ksni),
//! which KDE, Xfce, Cinnamon and GNOME-with-AppIndicator understand.
//!
//! On Windows the menu stays open while you pick settings, so you can try several in a row;
//! it closes when you click outside it (or on Show / hide, Help or Quit).

use winit::event_loop::EventLoopProxy;

use bytepet::look::{preset_name, Look, Part, PRESETS};
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
    look: Look,
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
            look: s.look,
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
            // ticked only while the look is still that preset's
            Choice::Shell(i) => PRESETS.get(i as usize).is_some_and(|p| p.name == preset_name(&self.look, self.shell)),
            Choice::Look(f, i) => self.look.get(f) == Some(i),
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

/// A settings submenu: options to pick one of, or more submenus.
enum Sub {
    Picks(Vec<(String, Choice)>),
    Subs(Vec<(String, Sub)>),
}

/// The settings submenus, each with its title.
fn submenus() -> Vec<(String, Sub)> {
    let mut sound = vec![("Off".to_string(), Choice::Sound(None))];
    sound.extend(VOLUMES.iter().map(|&v| (format!("{v}%"), Choice::Sound(Some(v)))));
    // Customize > Body > Color > Pink, ...
    let parts = Part::ALL
        .iter()
        .map(|&p| {
            let fields = p
                .fields()
                .iter()
                .map(|&f| {
                    let options = (0..f.count()).map(|i| (title_case(f.option_name(i)), Choice::Look(f, i))).collect();
                    (f.title().to_string(), Sub::Picks(options))
                })
                .collect();
            (title_case(p.name()), Sub::Subs(fields))
        })
        .collect();
    vec![
        (
            "Size".into(),
            Sub::Picks(SIZES.iter().enumerate().map(|(i, s)| (title_case(s.0), Choice::Size(i))).collect()),
        ),
        (
            "Shell".into(),
            Sub::Picks(
                PRESETS.iter().enumerate().map(|(i, s)| (title_case(s.name), Choice::Shell(i as u32))).collect(),
            ),
        ),
        ("Customize".into(), Sub::Subs(parts)),
        ("Speed".into(), Sub::Picks(SPEEDS.iter().map(|&x| (format!("{x}\u{d7}"), Choice::Speed(x))).collect())),
        ("Sound".into(), Sub::Picks(sound)),
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

/// A submenu and everything under it, collecting its check items.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn build_sub(
    title: &str,
    sub: Sub,
    state: &TrayState,
    picks: &mut Vec<(tray_icon::menu::CheckMenuItem, Choice)>,
) -> Option<tray_icon::menu::Submenu> {
    use tray_icon::menu::{CheckMenuItem, Submenu};
    let menu = Submenu::new(title, true);
    match sub {
        Sub::Picks(options) => {
            for (label, c) in options {
                let item = CheckMenuItem::with_id(menu_id(TrayCmd::Pick(c)), label, true, state.picked(c), None);
                menu.append(&item).ok()?;
                picks.push((item, c));
            }
        }
        Sub::Subs(subs) => {
            for (t, s) in subs {
                menu.append(&build_sub(&t, s, state, picks)?).ok()?;
            }
        }
    }
    Some(menu)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
impl Tray {
    /// Must be called on the main thread after the event loop has started.
    pub fn new(proxy: EventLoopProxy<UserEvent>, icon: (Vec<u8>, u32, u32), state: &TrayState) -> Option<Tray> {
        use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

        let menu = Menu::new();
        menu.append(&MenuItem::with_id(menu_id(TrayCmd::Toggle), "Show / hide Bytepet", true, None)).ok()?;
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        let mut picks = Vec::new();
        for (title, sub) in submenus() {
            menu.append(&build_sub(&title, sub, state, &mut picks)?).ok()?;
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

        // every item's command, in the order the items were added
        let mut cmds = vec![TrayCmd::Toggle];
        cmds.extend(picks.iter().map(|&(_, c)| TrayCmd::Pick(c)));
        cmds.extend(flags.iter().map(|&(_, f)| TrayCmd::Flip(f)));
        cmds.extend([TrayCmd::Help, TrayCmd::Quit]);
        #[cfg(target_os = "windows")]
        {
            use tray_icon::menu::ContextMenu;
            sticky::install(menu.hpopupmenu(), &cmds, proxy.clone());
        }
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

/// Windows closes a menu as soon as an item is clicked. A hook on the menu's message loop sees
/// a click on a setting first, carries it out and swallows it, so the menu stays open; clicks
/// elsewhere, and the other items, behave as usual.
#[cfg(target_os = "windows")]
mod sticky {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, CheckMenuItem, GetClassNameW, GetMenuItemCount, GetMenuItemInfoW, GetWindowThreadProcessId,
        MenuItemFromPoint, SendMessageW, SetWindowsHookExW, WindowFromPoint, HMENU, MENUITEMINFOW, MFS_CHECKED,
        MFS_DISABLED, MFT_SEPARATOR, MF_BYPOSITION, MF_CHECKED, MF_UNCHECKED, MIIM_FTYPE, MIIM_ID, MIIM_STATE,
        MIIM_SUBMENU, MN_GETHMENU, MSG, MSGF_MENU, WH_MSGFILTER, WM_LBUTTONUP,
    };
    use winit::event_loop::EventLoopProxy;

    use super::{TrayCmd, UserEvent};

    /// The window class of every popup menu.
    const MENU_CLASS: &str = "#32768";

    struct Sticky {
        /// Menu item ids and what they do.
        cmds: HashMap<u32, TrayCmd>,
        proxy: EventLoopProxy<UserEvent>,
    }

    thread_local! {
        static STICKY: RefCell<Option<Sticky>> = const { RefCell::new(None) };
    }

    /// `root` is the tray's menu and `cmds` its items' commands, depth first in the order they
    /// were added. Must be called on the thread that shows the menu.
    pub fn install(root: isize, cmds: &[TrayCmd], proxy: EventLoopProxy<UserEvent>) {
        let mut ids = Vec::new();
        unsafe { leaf_ids(root as HMENU, &mut ids) };
        if ids.len() != cmds.len() {
            // not laid out as expected: leave the menu to close as usual
            return;
        }
        let cmds = ids.into_iter().zip(cmds.iter().copied()).collect();
        STICKY.with(|s| *s.borrow_mut() = Some(Sticky { cmds, proxy }));
        unsafe { SetWindowsHookExW(WH_MSGFILTER, Some(hook), null_mut(), GetCurrentThreadId()) };
    }

    unsafe fn item(menu: HMENU, pos: i32) -> Option<MENUITEMINFOW> {
        let mut info: MENUITEMINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_ID | MIIM_SUBMENU | MIIM_FTYPE | MIIM_STATE;
        (GetMenuItemInfoW(menu, pos as u32, 1, &mut info) != 0).then_some(info)
    }

    /// The ids of every item that isn't a submenu or a separator, depth first.
    unsafe fn leaf_ids(menu: HMENU, out: &mut Vec<u32>) {
        for pos in 0..GetMenuItemCount(menu) {
            match item(menu, pos) {
                Some(info) if !info.hSubMenu.is_null() => leaf_ids(info.hSubMenu, out),
                Some(info) if info.fType & MFT_SEPARATOR == 0 => out.push(info.wID),
                _ => {}
            }
        }
    }

    /// The menu shown in a window, if it is one of this thread's popup menus.
    unsafe fn menu_of(win: HWND) -> Option<HMENU> {
        if win.is_null() || GetWindowThreadProcessId(win, null_mut()) != GetCurrentThreadId() {
            return None;
        }
        let mut name = [0u16; 16];
        let n = GetClassNameW(win, name.as_mut_ptr(), name.len() as i32).max(0) as usize;
        if String::from_utf16_lossy(&name[..n]) != MENU_CLASS {
            return None;
        }
        let menu = SendMessageW(win, MN_GETHMENU, 0, 0) as HMENU;
        (!menu.is_null()).then_some(menu)
    }

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == MSGF_MENU as i32 && lparam != 0 && on_menu_message(&*(lparam as *const MSG)) {
            return 1;
        }
        CallNextHookEx(null_mut(), code, wparam, lparam)
    }

    /// Is this a click on a setting? Then tick it, send it on and report it handled.
    unsafe fn on_menu_message(msg: &MSG) -> bool {
        if msg.message != WM_LBUTTONUP {
            return false;
        }
        let win = WindowFromPoint(msg.pt);
        let Some(menu) = menu_of(win) else { return false };
        let pos = MenuItemFromPoint(win, menu, msg.pt);
        let Some(info) = (pos >= 0)
            .then(|| item(menu, pos))
            .flatten()
            .filter(|i| i.hSubMenu.is_null() && i.fState & MFS_DISABLED == 0)
        else {
            return false;
        };
        let Some((cmd, proxy)) =
            STICKY.with(|s| s.borrow().as_ref().and_then(|s| Some((*s.cmds.get(&info.wID)?, s.proxy.clone()))))
        else {
            return false;
        };
        let check = |on: bool| MF_BYPOSITION | if on { MF_CHECKED } else { MF_UNCHECKED };
        match cmd {
            // a submenu of choices has one ticked: this one now
            TrayCmd::Pick(_) => {
                for i in 0..GetMenuItemCount(menu) {
                    CheckMenuItem(menu, i as u32, check(i == pos));
                }
            }
            TrayCmd::Flip(_) => {
                CheckMenuItem(menu, pos as u32, check(info.fState & MFS_CHECKED == 0));
            }
            // Show / hide, Help and Quit close the menu
            _ => return false,
        }
        InvalidateRect(win, null(), 1);
        let _ = proxy.send_event(UserEvent::Tray(cmd));
        true
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

/// A submenu and everything under it, as radio groups.
#[cfg(target_os = "linux")]
fn sni_sub(title: String, sub: Sub, state: &TrayState) -> ksni::MenuItem<SniTray> {
    use ksni::menu::{RadioGroup, RadioItem, SubMenu};
    let submenu = match sub {
        Sub::Picks(options) => {
            // nothing is ticked if the save holds a value the menu doesn't offer
            let selected = options.iter().position(|&(_, c)| state.picked(c)).unwrap_or(usize::MAX);
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
            vec![group.into()]
        }
        Sub::Subs(subs) => subs.into_iter().map(|(t, s)| sni_sub(t, s, state)).collect(),
    };
    SubMenu { label: title, submenu, ..Default::default() }.into()
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
        use ksni::menu::{CheckmarkItem, StandardItem};
        let send = |cmd: TrayCmd| {
            move |t: &mut SniTray| {
                let _ = t.proxy.send_event(UserEvent::Tray(cmd));
            }
        };
        let item = |label: &str, cmd: TrayCmd| -> ksni::MenuItem<SniTray> {
            StandardItem { label: label.into(), activate: Box::new(send(cmd)), ..Default::default() }.into()
        };
        let mut menu = vec![item("Show / hide Bytepet", TrayCmd::Toggle), ksni::MenuItem::Separator];
        for (title, sub) in submenus() {
            menu.push(sni_sub(title, sub, &self.state));
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
