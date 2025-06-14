use crate::core_graphics::*;
use crate::geometry::Rect;
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

type CFTypeRef = *const c_void;
pub type AXUIElementRef = CFTypeRef;
type CFStringRef = *const c_void;
type CFArrayRef = *const c_void;
type CFIndex = isize;
type PidT = i32;

#[repr(u32)]
enum AXValueType {
    CGPoint = 1,
    CGSize = 2,
}

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
    fn CFRelease(cf: CFTypeRef);
    fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
}

const KAX_WINDOWS: &str = "AXWindows";
const KAX_POSITION: &str = "AXPosition";
const KAX_SIZE: &str = "AXSize";
const KCF_STRING_ENCODING_UTF8: u32 = 0x08000100;

fn cfstring(s: &str) -> CFStringRef {
    let cstr = CString::new(s).unwrap();
    unsafe { CFStringCreateWithCString(ptr::null(), cstr.as_ptr(), KCF_STRING_ENCODING_UTF8) }
}

pub struct Window {
    pub ax_ref: AXUIElementRef,
    pub app_name: String,
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.ax_ref);
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
            let mut windows_ref: CFTypeRef = ptr::null();

            let result =
                AXUIElementCopyAttributeValue(app_ax, cfstring(KAX_WINDOWS), &mut windows_ref);
            if result != 0 || windows_ref.is_null() {
                continue;
            }

            let window_count = CFArrayGetCount(windows_ref as CFArrayRef);

            for j in 0..window_count {
                let window = CFArrayGetValueAtIndex(windows_ref as CFArrayRef, j) as AXUIElementRef;
                let retained_window = CFRetain(window) as AXUIElementRef;

                windows.push(Window {
                    ax_ref: retained_window,
                    app_name: app_name.clone(),
                });
            }

            CFRelease(windows_ref);
        }
    }

    windows
}

pub fn move_and_resize_window(window: &Window, rect: Rect) {
    unsafe {
        let pos = CGPoint {
            x: rect.x,
            y: rect.y,
        };
        let pos_value = AXValueCreate(AXValueType::CGPoint, &pos as *const _ as *const c_void);
        let pos_result =
            AXUIElementSetAttributeValue(window.ax_ref, cfstring(KAX_POSITION), pos_value);
        CFRelease(pos_value);

        let size = CGSize {
            width: rect.width,
            height: rect.height,
        };
        let size_value = AXValueCreate(AXValueType::CGSize, &size as *const _ as *const c_void);
        let size_result =
            AXUIElementSetAttributeValue(window.ax_ref, cfstring(KAX_SIZE), size_value);
        CFRelease(size_value);

        if pos_result != 0 || size_result != 0 {
            println!(
                "Failed to move/resize window for app: {} (pos_result: {}, size_result: {})",
                window.app_name, pos_result, size_result
            );
        }
    }
}

pub fn window_rect(window: &Window) -> Option<Rect> {
    unsafe {
        let mut pos_ref: CFTypeRef = std::ptr::null();
        let mut size_ref: CFTypeRef = std::ptr::null();

        let pos_result =
            AXUIElementCopyAttributeValue(window.ax_ref, cfstring(KAX_POSITION), &mut pos_ref);
        let size_result =
            AXUIElementCopyAttributeValue(window.ax_ref, cfstring(KAX_SIZE), &mut size_ref);

        if pos_result != 0 || size_result != 0 || pos_ref.is_null() || size_ref.is_null() {
            println!(
                "Failed to get rect for '{}': pos_result={}, size_result={}, pos_ref={:?}, size_ref={:?}",
                window.app_name, pos_result, size_result, pos_ref, size_ref
            );
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
            println!(
                "AXValueGetValue failed for '{}': got_pos={}, got_size={}",
                window.app_name, got_pos, got_size
            );
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
