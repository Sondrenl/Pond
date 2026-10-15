use objc2::runtime::{AnyClass, AnyObject, Bool, Sel};
use objc2_app_kit::{NSColor, NSWindow, NSWindowCollectionBehavior};
use std::ffi::c_char;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

extern "C" {
    fn object_getClass(obj: *mut AnyObject) -> *mut AnyClass;
    fn class_replaceMethod(
        cls: *mut AnyClass,
        name: Sel,
        imp: unsafe extern "C" fn(*mut AnyObject, Sel) -> Bool,
        types: *const c_char,
    ) -> usize;
}

/// Trampoline returning NO for key/main window queries.
unsafe extern "C" fn cannot_become_key(_this: *mut AnyObject, _cmd: Sel) -> Bool {
    Bool::NO
}

/// Target desktop window level: strictly below normal application windows.
/// On macOS, kCGNormalWindowLevel is 0. Normal apps (terminals, browsers, etc.)
/// live at levels >= 0. Setting level to -1 places the window strictly beneath them.
pub const DESKTOP_WINDOW_LEVEL: isize = -1;

pub fn configure_macos_window(window: &Window) {
    let raw_handle = window
        .window_handle()
        .expect("Failed to obtain raw window handle")
        .as_raw();

    let ns_view_ptr: *mut AnyObject = match raw_handle {
        RawWindowHandle::AppKit(handle) => handle.ns_view.as_ptr().cast(),
        _ => panic!("Expected AppKit window handle on macOS"),
    };

    unsafe {
        // Query the NSWindow from the NSView
        let ns_window_ptr: *mut AnyObject = objc2::msg_send![ns_view_ptr, window];
        assert!(
            !ns_window_ptr.is_null(),
            "NSWindow pointer must not be null"
        );

        let ns_window: &NSWindow = &*(ns_window_ptr as *const NSWindow);

        // 1. Prevent the window from stealing focus or becoming key window
        let cls = object_getClass(ns_window_ptr);
        let sel_key = objc2::sel!(canBecomeKeyWindow);
        let sel_main = objc2::sel!(canBecomeMainWindow);
        let types = c"c@:".as_ptr();
        class_replaceMethod(cls, sel_key, cannot_become_key, types);
        class_replaceMethod(cls, sel_main, cannot_become_key, types);

        // 2. Window Level: Stay strictly behind normal windows
        ns_window.setLevel(DESKTOP_WINDOW_LEVEL);

        // 3. Collection Behavior: Visible across all Spaces & stationary during Mission Control
        let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary;
        ns_window.setCollectionBehavior(behavior);

        // 4. Borderless, transparent, non-resizable background styling
        ns_window.setOpaque(false);
        ns_window.setHasShadow(false);
        let clear_color = NSColor::clearColor();
        ns_window.setBackgroundColor(Some(&clear_color));

        // Ensure layer is also transparent
        let view_layer: *mut AnyObject = objc2::msg_send![ns_view_ptr, layer];
        if !view_layer.is_null() {
            let _: () = objc2::msg_send![view_layer, setOpaque: Bool::NO];
        }

        // 5. Background persistence & mouse interaction
        ns_window.setHidesOnDeactivate(false);
        ns_window.setCanHide(false);
        ns_window.setIgnoresMouseEvents(false);
    }
}
