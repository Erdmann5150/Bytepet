//! The tray / menu-bar icon: click to show or hide the pet. Its menu is kept short: show or
//! hide, Settings (which opens them on the device's screen) and Quit. Windows and macOS use
//! tray-icon; Linux uses the StatusNotifierItem D-Bus protocol (ksni), which KDE, Xfce,
//! Cinnamon and GNOME-with-AppIndicator understand. Hovering it shows how the pet is doing
//! (see `Core::summary`).

use winit::event_loop::EventLoopProxy;

/// The tooltip's heading, above the pet's status.
const TITLE: &str = "Bytepet";

/// Windows cuts tooltips off at 127 characters.
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn tooltip(status: &str) -> String {
    format!("{TITLE}\n{status}").chars().take(127).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCmd {
    Toggle,
    /// Show the pet with its settings open.
    Settings,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserEvent {
    Tray(TrayCmd),
    /// Bytepet was launched again with the same save.
    Show,
}

/// The menu's items, in order; `None` is a separator.
const ITEMS: [Option<(&str, TrayCmd)>; 4] = [
    Some(("Show / hide Bytepet", TrayCmd::Toggle)),
    Some(("Settings", TrayCmd::Settings)),
    None,
    Some(("Quit Bytepet", TrayCmd::Quit)),
];

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
    icon: tray_icon::TrayIcon,
    status: String,
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn menu_id(cmd: TrayCmd) -> String {
    format!("{cmd:?}")
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
impl Tray {
    /// Must be called on the main thread after the event loop has started.
    pub fn new(proxy: EventLoopProxy<UserEvent>, icon: (Vec<u8>, u32, u32)) -> Option<Tray> {
        use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
        use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

        let menu = Menu::new();
        for item in ITEMS {
            match item {
                Some((label, cmd)) => menu.append(&MenuItem::with_id(menu_id(cmd), label, true, None)).ok()?,
                None => menu.append(&PredefinedMenuItem::separator()).ok()?,
            }
        }
        let p1 = proxy.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Some((_, cmd)) = ITEMS.iter().flatten().find(|(_, c)| e.id == menu_id(*c).as_str()) {
                let _ = p1.send_event(UserEvent::Tray(*cmd));
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
            .with_tooltip(TITLE)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .ok()?;
        Some(Tray { icon: tray, status: String::new() })
    }

    /// Show the pet's status on hover. Cheap to call every frame: only a change reaches the OS.
    pub fn set_status(&mut self, status: &str) {
        if self.status != status {
            self.status = status.to_string();
            let _ = self.icon.set_tooltip(Some(tooltip(status)));
        }
    }
}

// ====================================================================== Linux
#[cfg(target_os = "linux")]
pub struct Tray {
    handle: ksni::blocking::Handle<SniTray>,
    status: String,
}

#[cfg(target_os = "linux")]
pub struct SniTray {
    proxy: EventLoopProxy<UserEvent>,
    icon: (Vec<u8>, u32, u32),
    status: String,
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
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip { title: TITLE.into(), description: self.status.clone(), ..Default::default() }
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.proxy.send_event(UserEvent::Tray(TrayCmd::Toggle));
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        ITEMS
            .iter()
            .map(|item| match *item {
                Some((label, cmd)) => StandardItem {
                    label: label.into(),
                    activate: Box::new(move |t: &mut SniTray| {
                        let _ = t.proxy.send_event(UserEvent::Tray(cmd));
                    }),
                    ..Default::default()
                }
                .into(),
                None => ksni::MenuItem::Separator,
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
impl Tray {
    pub fn new(proxy: EventLoopProxy<UserEvent>, icon: (Vec<u8>, u32, u32)) -> Option<Tray> {
        use ksni::blocking::TrayMethods;
        // If no tray host is running (stock GNOME), the icon simply never appears; the window
        // stays in the taskbar, so nothing is lost.
        let tray = SniTray { proxy, icon, status: String::new() };
        let handle = tray.assume_sni_available(true).spawn().ok()?;
        Some(Tray { handle, status: String::new() })
    }

    /// Show the pet's status on hover. Cheap to call every frame: only a change goes out
    /// over D-Bus.
    pub fn set_status(&mut self, status: &str) {
        if self.status != status {
            self.status = status.to_string();
            let s = self.status.clone();
            self.handle.update(move |t| t.status = s);
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub struct Tray;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
impl Tray {
    pub fn new(_proxy: EventLoopProxy<UserEvent>, _icon: (Vec<u8>, u32, u32)) -> Option<Tray> {
        None
    }
    pub fn set_status(&mut self, _status: &str) {}
}
