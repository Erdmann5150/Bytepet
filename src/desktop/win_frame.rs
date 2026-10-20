//! Windows: stop the system painting a title bar over the frameless window.
//!
//! winit keeps WS_CAPTION on frameless windows (for snapping and animations) and hides the frame
//! by answering WM_NCCALCSIZE. Our window region (see `shape`) turns off the DWM-drawn frame, so
//! whenever focus changes Windows falls back to painting a classic title bar across the top of
//! the shell. We hook the window procedure and skip those paints; everything else goes to winit.

use std::sync::atomic::{AtomicIsize, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GetWindowLongPtrW, SetWindowLongPtrW, GWLP_WNDPROC, WM_NCACTIVATE, WM_NCPAINT, WNDPROC,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

/// Undocumented messages the theme engine sends to draw the caption and frame.
const WM_NCUAHDRAWCAPTION: u32 = 0x00AE;
const WM_NCUAHDRAWFRAME: u32 = 0x00AF;

/// winit's window procedure, which ours hands every other message to.
static WINIT_PROC: AtomicIsize = AtomicIsize::new(0);

pub fn install(window: &Window) {
    let Ok(h) = window.window_handle() else { return };
    let RawWindowHandle::Win32(h) = h.as_raw() else { return };
    let hwnd = h.hwnd.get() as HWND;
    if WINIT_PROC.load(Ordering::SeqCst) != 0 {
        return; // there is only ever one window
    }
    unsafe {
        WINIT_PROC.store(GetWindowLongPtrW(hwnd, GWLP_WNDPROC), Ordering::SeqCst);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, frameless_proc as *const () as isize);
    }
}

unsafe extern "system" fn frameless_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let winit: WNDPROC = std::mem::transmute(WINIT_PROC.load(Ordering::SeqCst));
    match msg {
        // winit still hears about the focus change; an lParam of -1 tells the default
        // handler not to repaint the frame for it
        WM_NCACTIVATE => CallWindowProcW(winit, hwnd, msg, wparam, -1),
        // there is no frame to paint
        WM_NCPAINT | WM_NCUAHDRAWCAPTION | WM_NCUAHDRAWFRAME => 0,
        _ => CallWindowProcW(winit, hwnd, msg, wparam, lparam),
    }
}
