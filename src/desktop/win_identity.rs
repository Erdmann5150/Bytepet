//! Windows names a notification after the app that sent it. A program that isn't installed
//! as a package has no identity of its own, so without this, toasts would have to borrow one
//! (PowerShell's, which looks suspicious). Registering an AppUserModelID under the current
//! user with a display name and icon makes them show as "Bytepet" with the pet's icon.
//! No admin rights needed; nothing outside HKCU and the save folder is touched.

use std::os::windows::ffi::OsStrExt;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE,
    REG_SZ,
};

pub const AUMID: &str = "Bytepet.Desktop";
const ICON: &[u8] = include_bytes!("../../assets/icon.png");

static REGISTERED: OnceLock<bool> = OnceLock::new();

/// The ID to send notifications under, or `None` to fall back to the default if registering
/// failed (a toast under an unregistered ID might not show at all).
pub fn app_id() -> Option<&'static str> {
    REGISTERED.get_or_init(register).then_some(AUMID)
}

fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn register() -> bool {
    // the icon must be a file on disk; keep a copy next to the save
    let icon = bytepet::save::data_dir().join("bytepet-icon.png");
    let current = std::fs::metadata(&icon).map(|m| m.len() == ICON.len() as u64).unwrap_or(false);
    if !current
        && std::fs::create_dir_all(icon.parent().unwrap_or(&icon)).and_then(|_| std::fs::write(&icon, ICON)).is_err()
    {
        return false;
    }
    let key = wide(format!("Software\\Classes\\AppUserModelId\\{AUMID}").as_ref());
    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        let created = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut hkey,
            std::ptr::null_mut(),
        );
        if created != ERROR_SUCCESS {
            return false;
        }
        let mut ok = true;
        for (name, value) in [("DisplayName", std::ffi::OsStr::new("Bytepet")), ("IconUri", icon.as_os_str())] {
            let (name, value) = (wide(name.as_ref()), wide(value));
            let set =
                RegSetValueExW(hkey, name.as_ptr(), 0, REG_SZ, value.as_ptr() as *const u8, (value.len() * 2) as u32);
            ok &= set == ERROR_SUCCESS;
        }
        RegCloseKey(hkey);
        ok
    }
}
