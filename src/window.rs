use crate::core_graphics::*;
use crate::geometry::Rect;
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::os::raw::{c_char, c_void};
use std::ptr;

pub type AXUIElementRef = *const c_void;
type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFArrayRef = *const c_void;
type CFIndex = isize;
type PidT = i32;

#[derive(Debug, Copy, Clone)]
pub struct SendableAXUIElementRef(pub AXUIElementRef);

unsafe impl Send for SendableAXUIElementRef {}
unsafe impl Sync for SendableAXUIElementRef {}

impl Deref for SendableAXUIElementRef {
    type Target = AXUIElementRef;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[repr(u32)]
enum AXValueType {
    CGPoint = 1,
    CGSize = 2,
}

#[allow(improper_ctypes)]
unsafe extern "C" {
    fn AXUIElementCreateApplication(pid: PidT) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXValueCreate(theType: AXValueType, valuePtr: *const c_void) -> CFTypeRef;
    fn AXValueGetValue(value: CFTypeRef, theType: AXValueType, valuePtr: *mut c_void) -> i8;
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: CFIndex) -> CFTypeRef;
    fn CFStringCreateWithCString(
        alloc: CFTypeRef,
        cStr: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    fn CFBooleanGetValue(boolean: CFTypeRef) -> u8;
    pub fn CFEqual(cf1: CFTypeRef, cf2: CFTypeRef) -> u8;
}

const KAX_WINDOWS: &str = "AXWindows";
const KAX_POSITION: &str = "AXPosition";
const KAX_SIZE: &str = "AXSize";
const KAX_MINIMIZED: &str = "AXMinimized";
const KAX_FOCUSED_WINDOW: &str = "AXFocusedWindow";
const KAX_TITLE: &str = "AXTitle";
const KCF_STRING_ENCODING_UTF8: u32 = 0x08000100;

fn cfstring(s: &str) -> CFStringRef {
    let cstr = CString::new(s).unwrap();
    unsafe { CFStringCreateWithCString(ptr::null(), cstr.as_ptr(), KCF_STRING_ENCODING_UTF8) }
}

pub struct Window {
    pub ax_ref: SendableAXUIElementRef,
    pub app_name: String,
    pub title: String,
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        unsafe { CFEqual(*self.ax_ref, *other.ax_ref) != 0 }
    }
}
impl Eq for Window {}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            CFRelease(*self.ax_ref);
        }
    }
}

pub fn get_window_title(element: AXUIElementRef) -> String {
    unsafe {
        let mut title_ref: CFTypeRef = ptr::null();
        let attr = cfstring(KAX_TITLE);
        let result = AXUIElementCopyAttributeValue(element, attr, &mut title_ref);
        CFRelease(attr);

        if result == 0 && !title_ref.is_null() {
            let value_type_id: usize = msg_send![title_ref as *const AnyObject, _cfTypeID];

            // CFStringGetTypeID() is 7
            if value_type_id == 7 {
                let c_str: *const c_char = msg_send![title_ref as *const AnyObject, UTF8String];
                if !c_str.is_null() {
                    let title = CStr::from_ptr(c_str).to_string_lossy().into_owned();
                    CFRelease(title_ref);
                    return title;
                }
            }
            CFRelease(title_ref);
        }
        "<Untitled>".to_string()
    }
}

pub fn get_focused_window_ref() -> Option<SendableAXUIElementRef> {
    unsafe {
        let nsworkspace_cstr = CStr::from_bytes_with_nul_unchecked(b"NSWorkspace\0");
        let nsworkspace = AnyClass::get(nsworkspace_cstr).expect("NSWorkspace class not found");
        let shared_workspace: *mut AnyObject = msg_send![nsworkspace, sharedWorkspace];
        let frontmost_app: *mut AnyObject = msg_send![shared_workspace, frontmostApplication];

        if frontmost_app.is_null() {
            return None;
        }

        let pid: i32 = msg_send![frontmost_app, processIdentifier];
        let app_ax = AXUIElementCreateApplication(pid);
        if app_ax.is_null() {
            return None;
        }

        let mut focused_window_ref: CFTypeRef = ptr::null();
        let focused_window_attr = cfstring(KAX_FOCUSED_WINDOW);
        let result =
            AXUIElementCopyAttributeValue(app_ax, focused_window_attr, &mut focused_window_ref);
        CFRelease(focused_window_attr);
        CFRelease(app_ax);

        if result == 0 && !focused_window_ref.is_null() {
            Some(SendableAXUIElementRef(focused_window_ref))
        } else {
            if !focused_window_ref.is_null() {
                CFRelease(focused_window_ref)
            };
            None
        }
    }
}

pub fn collect_windows() -> Vec<Window> {
    let mut windows = Vec::new();
    unsafe {
        let nsworkspace_cstr = CStr::from_bytes_with_nul_unchecked(b"NSWorkspace\0");
        let nsworkspace = AnyClass::get(nsworkspace_cstr).expect("NSWorkspace class not found");
        let shared_workspace: *mut AnyObject = msg_send![nsworkspace, sharedWorkspace];
        let running_apps: *mut AnyObject = msg_send![shared_workspace, runningApplications];
        let count: usize = msg_send![running_apps, count];

        for i in 0..count {
            let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
            let name: *mut AnyObject = msg_send![app, localizedName];
            let cstr: *const c_char = msg_send![name, UTF8String];

            let app_name = if !cstr.is_null() {
                CStr::from_ptr(cstr).to_string_lossy().into_owned()
            } else {
                "<unknown>".to_string()
            };
            if app_name == "Finder" || app_name == "Dock" {
                continue;
            }

            let pid: i32 = msg_send![app, processIdentifier];
            let app_ax = AXUIElementCreateApplication(pid);
            if app_ax.is_null() {
                continue;
            }

            let mut windows_ref: CFTypeRef = ptr::null();
            let windows_attr = cfstring(KAX_WINDOWS);
            let result = AXUIElementCopyAttributeValue(app_ax, windows_attr, &mut windows_ref);
            CFRelease(windows_attr);

            if result != 0 || windows_ref.is_null() {
                CFRelease(app_ax);
                continue;
            }

            let window_count = CFArrayGetCount(windows_ref as CFArrayRef);
            for j in 0..window_count {
                let window_ref =
                    CFArrayGetValueAtIndex(windows_ref as CFArrayRef, j) as AXUIElementRef;
                let retained_window_ref = CFRetain(window_ref) as AXUIElementRef;

                let title = get_window_title(retained_window_ref);
                if title.is_empty() {
                    CFRelease(retained_window_ref);
                    continue;
                }

                windows.push(Window {
                    ax_ref: SendableAXUIElementRef(retained_window_ref),
                    app_name: app_name.clone(),
                    title,
                });
            }
            CFRelease(windows_ref);
            CFRelease(app_ax);
        }
    }
    windows
}

pub fn move_and_resize_window(window: &Window, rect: Rect) {
    unsafe {
        let pos_attr = cfstring(KAX_POSITION);
        let size_attr = cfstring(KAX_SIZE);
        let pos = CGPoint {
            x: rect.x,
            y: rect.y,
        };
        let pos_value = AXValueCreate(AXValueType::CGPoint, &pos as *const _ as *const c_void);
        AXUIElementSetAttributeValue(*window.ax_ref, pos_attr, pos_value);
        CFRelease(pos_value);
        let size = CGSize {
            width: rect.width,
            height: rect.height,
        };
        let size_value = AXValueCreate(AXValueType::CGSize, &size as *const _ as *const c_void);
        AXUIElementSetAttributeValue(*window.ax_ref, size_attr, size_value);
        CFRelease(size_value);
        CFRelease(pos_attr);
        CFRelease(size_attr);
    }
}

pub fn window_rect(window: &Window) -> Option<Rect> {
    unsafe {
        let mut pos_ref: CFTypeRef = ptr::null();
        let mut size_ref: CFTypeRef = ptr::null();
        let pos_attr = cfstring(KAX_POSITION);
        let size_attr = cfstring(KAX_SIZE);
        let pos_result = AXUIElementCopyAttributeValue(*window.ax_ref, pos_attr, &mut pos_ref);
        let size_result = AXUIElementCopyAttributeValue(*window.ax_ref, size_attr, &mut size_ref);
        CFRelease(pos_attr);
        CFRelease(size_attr);

        if pos_result != 0 || size_result != 0 || pos_ref.is_null() || size_ref.is_null() {
            if !pos_ref.is_null() {
                CFRelease(pos_ref)
            };
            if !size_ref.is_null() {
                CFRelease(size_ref)
            };
            return None;
        }

        let mut pos = CGPoint { x: 0.0, y: 0.0 };
        let mut size = CGSize {
            width: 0.0,
            height: 0.0,
        };
        let got_pos = AXValueGetValue(
            pos_ref,
            AXValueType::CGPoint,
            &mut pos as *mut _ as *mut c_void,
        );
        let got_size = AXValueGetValue(
            size_ref,
            AXValueType::CGSize,
            &mut size as *mut _ as *mut c_void,
        );
        CFRelease(pos_ref);
        CFRelease(size_ref);

        if got_pos == 0 || got_size == 0 {
            return None;
        }
        Some(Rect {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        })
    }
}

pub fn is_window_minimized(window: &Window) -> bool {
    unsafe {
        let mut value_ref: CFTypeRef = ptr::null();
        let attr = cfstring(KAX_MINIMIZED);
        let result = AXUIElementCopyAttributeValue(*window.ax_ref, attr, &mut value_ref);
        CFRelease(attr);

        if result != 0 || value_ref.is_null() {
            if !value_ref.is_null() {
                CFRelease(value_ref);
            }
            return false;
        }

        let is_minimized = CFBooleanGetValue(value_ref) != 0;
        CFRelease(value_ref);
        is_minimized
    }
}
