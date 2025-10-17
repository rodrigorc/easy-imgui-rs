/// Easy wrapper for the clipboard functions.
///
/// This module depends on the `clipboard` feature. Usually this is set up automatically just by
/// enabling the feature.
use std::ffi::{CStr, CString, c_char, c_void};

/// Sets up the ImGui clipboard using the `arboard` crate.
pub fn setup(imgui: &mut super::Context) {
    if let Ok(ctx) = arboard::Clipboard::new() {
        let clip = MyClipboard {
            ctx,
            text: CString::default(),
        };
        unsafe {
            let pio = imgui.platform_io_mut();
            pio.Platform_ClipboardUserData = Box::into_raw(Box::new(clip)) as *mut c_void;
            pio.Platform_SetClipboardTextFn = Some(set_clipboard_text);
            pio.Platform_GetClipboardTextFn = Some(get_clipboard_text);
        }
    }
}

/// Releases the resources of a call to `setup`.
pub fn release(imgui: &mut super::Context) {
    unsafe {
        let pio = imgui.platform_io_mut();
        let Some(p) = pio.Platform_SetClipboardTextFn else {
            return;
        };
        if !std::ptr::fn_addr_eq(p, set_clipboard_text as unsafe extern "C" fn(_, _)) {
            return;
        }
        pio.Platform_SetClipboardTextFn = None;
        pio.Platform_GetClipboardTextFn = None;
        let _ = Box::from_raw(pio.Platform_ClipboardUserData as *mut MyClipboard);
        pio.Platform_ClipboardUserData = std::ptr::null_mut();
    }
}

unsafe extern "C" fn set_clipboard_text(
    imgui: *mut easy_imgui_sys::ImGuiContext,
    text: *const c_char,
) {
    unsafe {
        let user = (*imgui).PlatformIO.Platform_ClipboardUserData;
        let clip = &mut *(user as *mut MyClipboard);
        if text.is_null() {
            let _ = clip.ctx.clear();
        } else {
            let cstr = CStr::from_ptr(text);
            let str = String::from_utf8_lossy(cstr.to_bytes()).to_string();
            let _ = clip.ctx.set_text(str);
        }
    }
}

// The returned pointer should be valid for a while...
unsafe extern "C" fn get_clipboard_text(imgui: *mut easy_imgui_sys::ImGuiContext) -> *const c_char {
    unsafe {
        let user = (*imgui).PlatformIO.Platform_ClipboardUserData;
        let clip = &mut *(user as *mut MyClipboard);
        let Ok(text) = clip.ctx.get_text() else {
            return std::ptr::null();
        };
        let Ok(text) = CString::new(text) else {
            return std::ptr::null();
        };
        clip.text = text;
        clip.text.as_ptr()
    }
}

struct MyClipboard {
    ctx: arboard::Clipboard,
    text: CString,
}
