//! Make clicks on the transparent area around the egg fall through to the desktop.
//!
//! - Windows: a window region (SetWindowRgn) shaped like the device.
//! - Linux/X11: an XShape *input* region, so drawing (and anti-aliased edges) is untouched.
//! - macOS: fully transparent pixels already let clicks through, nothing to do.
//! - Wayland: not supported by winit; the corners stay clickable.

// Only Windows and X11 need the outline; macOS compiles the no-op path.
#![cfg_attr(target_os = "macos", allow(unused_imports, dead_code, unused_variables))]
#![cfg_attr(target_os = "windows", allow(unused_imports))]

use bytepet::layout::{egg_points, star_points, CX, CY, SHELL_A, SHELL_B, STAR, VIEW};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::window::Window;

type Poly = Vec<(i32, i32)>;

/// Device outline polygons in window pixels, for a scale of `s` pixels per design unit.
fn outlines(s: f32) -> (Poly, Poly, Poly, (i32, i32, i32, i32)) {
    let to_px = |(x, y): (f32, f32)| (((x - VIEW.0) * s).round() as i32, ((y - VIEW.1) * s).round() as i32);
    let egg: Vec<(i32, i32)> = egg_points(CX, CY, SHELL_A + 3.0, SHELL_B + 3.0, 120).into_iter().map(to_px).collect();
    // the star, grown a little, and the stem joining it to the shell
    let star: Vec<(i32, i32)> = star_points()
        .into_iter()
        .map(|(x, y)| (STAR.0 + (x - STAR.0) * 1.15, STAR.1 + (y - STAR.1) * 1.15))
        .map(to_px)
        .collect();
    let stem: Vec<(i32, i32)> = [(8.0, 16.0), (16.0, 8.0), (56.0, 48.0), (48.0, 56.0)]
        .into_iter()
        .map(|(dx, dy)| to_px((STAR.0 + dx, STAR.1 + dy)))
        .collect();
    let (rx0, ry0) = to_px((CX - 22.0, CY - SHELL_B - 28.0));
    let (rx1, ry1) = to_px((CX + 22.0, CY - SHELL_B + 16.0));
    (egg, star, stem, (rx0, ry0, rx1, ry1))
}

pub fn apply(window: &Window, scale: f32) {
    let Ok(wh) = window.window_handle() else { return };
    let Ok(dh) = window.display_handle() else { return };
    match (wh.as_raw(), dh.as_raw()) {
        #[cfg(target_os = "windows")]
        (RawWindowHandle::Win32(h), _) => windows_region(h.hwnd.get(), scale),
        #[cfg(all(unix, not(target_os = "macos")))]
        (RawWindowHandle::Xlib(w), RawDisplayHandle::Xlib(d)) => {
            if let Some(display) = d.display {
                x11_input_shape(
                    display.as_ptr(),
                    w.window,
                    scale,
                    window.inner_size().width,
                    window.inner_size().height,
                );
            }
        }
        _ => {}
    }
}

#[cfg(target_os = "windows")]
fn windows_region(hwnd: isize, scale: f32) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreateEllipticRgn, CreatePolygonRgn, DeleteObject, SetWindowRgn, RGN_OR, WINDING,
    };
    let (egg, star, stem, ring) = outlines(scale);
    let pts = |v: &[(i32, i32)]| v.iter().map(|&(x, y)| POINT { x, y }).collect::<Vec<_>>();
    unsafe {
        let e = pts(&egg);
        let region = CreatePolygonRgn(e.as_ptr(), e.len() as i32, WINDING);
        for part in [&star, &stem] {
            let p = pts(part);
            let r = CreatePolygonRgn(p.as_ptr(), p.len() as i32, WINDING);
            CombineRgn(region, region, r, RGN_OR);
            DeleteObject(r);
        }
        let ring_rgn = CreateEllipticRgn(ring.0, ring.1, ring.2, ring.3);
        CombineRgn(region, region, ring_rgn, RGN_OR);
        DeleteObject(ring_rgn);
        // the system owns `region` after this call
        SetWindowRgn(hwnd as _, region, 1);
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn x11_input_shape(display: *mut std::ffi::c_void, window: std::ffi::c_ulong, scale: f32, w: u32, h: u32) {
    use bytepet::layout::point_in_poly;
    #[repr(C)]
    struct XRectangle {
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    }
    type ShapeRects =
        unsafe extern "C" fn(*mut std::ffi::c_void, std::ffi::c_ulong, i32, i32, i32, *const XRectangle, i32, i32, i32);
    type Flush = unsafe extern "C" fn(*mut std::ffi::c_void) -> i32;
    const SHAPE_INPUT: i32 = 2;
    const SHAPE_SET: i32 = 0;
    const UNSORTED: i32 = 0;

    let (egg, star, stem, ring) = outlines(scale);
    let tf: Vec<(f32, f32)> = stem.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
    let ef: Vec<(f32, f32)> = egg.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
    let sf: Vec<(f32, f32)> = star.iter().map(|&(x, y)| (x as f32, y as f32)).collect();
    let (rcx, rcy, rr) =
        ((ring.0 + ring.2) as f32 / 2.0, (ring.1 + ring.3) as f32 / 2.0, (ring.2 - ring.0) as f32 / 2.0);
    let inside = |x: f32, y: f32| {
        point_in_poly(x, y, &ef)
            || point_in_poly(x, y, &sf)
            || point_in_poly(x, y, &tf)
            || ((x - rcx).powi(2) + (y - rcy).powi(2)).sqrt() <= rr
    };
    // runs of clickable pixels, two rows at a time
    let mut rects = Vec::new();
    let step = 2;
    for y in (0..h as i32).step_by(step) {
        let mut x = 0;
        while x < w as i32 {
            if inside(x as f32 + 0.5, y as f32 + 1.0) {
                let start = x;
                while x < w as i32 && inside(x as f32 + 0.5, y as f32 + 1.0) {
                    x += 1;
                }
                rects.push(XRectangle { x: start as i16, y: y as i16, width: (x - start) as u16, height: step as u16 });
            }
            x += 1;
        }
    }
    unsafe {
        let Ok(xext) = libloading::Library::new("libXext.so.6") else { return };
        let Ok(x11) = libloading::Library::new("libX11.so.6") else { return };
        let Ok(shape) = xext.get::<ShapeRects>(b"XShapeCombineRectangles\0") else { return };
        let Ok(flush) = x11.get::<Flush>(b"XFlush\0") else { return };
        shape(display, window, SHAPE_INPUT, 0, 0, rects.as_ptr(), rects.len() as i32, SHAPE_SET, UNSORTED);
        flush(display);
    }
}
