//! Launch at login: registry Run key (Windows), LaunchAgent (macOS), XDG autostart (Linux).

use auto_launch::{AutoLaunch, AutoLaunchBuilder};

fn launcher(save_arg: Option<&str>) -> Option<AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    let mut b = AutoLaunchBuilder::new();
    b.set_app_name("Bytepet").set_app_path(&exe.to_string_lossy());
    if let Some(p) = save_arg {
        b.set_args(&["--save", p]);
    }
    #[cfg(target_os = "macos")]
    b.set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent);
    #[cfg(target_os = "linux")]
    b.set_linux_launch_mode(auto_launch::LinuxLaunchMode::XdgAutostart);
    b.build().ok()
}

pub fn is_enabled() -> bool {
    launcher(None).and_then(|l| l.is_enabled().ok()).unwrap_or(false)
}

/// `save_arg` is passed along only when the user runs with a non-default save file.
pub fn set_enabled(on: bool, save_arg: Option<&str>) -> bool {
    match launcher(save_arg) {
        Some(l) => if on { l.enable() } else { l.disable() }.is_ok(),
        None => false,
    }
}
