#[link(name = "AppKit", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {}

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

use objc2::{
    class, msg_send,
    runtime::{AnyClass, AnyObject},
};

type CFTypeRef = *const c_void;
type AXUIElementRef = CFTypeRef;
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
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: CFIndex) -> CFTypeRef;
    fn CFStringCreateWithCString(
        alloc: CFTypeRef,
        cStr: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    fn CFRelease(cf: CFTypeRef);
}

const KAX_WINDOWS: &str = "AXWindows";
const KAX_POSITION: &str = "AXPosition";
const KAX_SIZE: &str = "AXSize";
const KCF_STRING_ENCODING_UTF8: u32 = 0x08000100;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct CGSize {
    width: f64,
    height: f64,
}

fn cfstring(s: &str) -> CFStringRef {
    let cstr = CString::new(s).unwrap();
    unsafe { CFStringCreateWithCString(ptr::null(), cstr.as_ptr(), KCF_STRING_ENCODING_UTF8) }
}

fn main() {
    unsafe {
        let pool: *mut AnyObject = msg_send![class!(NSAutoreleasePool), new];

        // Get NSWorkspace and running applications
        let nsworkspace_cstr = CStr::from_bytes_with_nul_unchecked(b"NSWorkspace\0");
        let nsworkspace = AnyClass::get(nsworkspace_cstr).expect("NSWorkspace class not found");
        let shared_workspace: *mut AnyObject = msg_send![nsworkspace, sharedWorkspace];
        let running_apps: *mut AnyObject = msg_send![shared_workspace, runningApplications];

        let count: usize = msg_send![running_apps, count];
        for i in 0..count {
            let app: *mut AnyObject = msg_send![running_apps, objectAtIndex: i];
            let name: *mut AnyObject = msg_send![app, localizedName];
            let cstr: *const c_char = msg_send![name, UTF8String];
            let rust_str = if !cstr.is_null() {
                CStr::from_ptr(cstr).to_string_lossy().into_owned()
            } else {
                "<unknown>".to_string()
            };

            // Skip Finder and Dock
            if rust_str == "Finder" || rust_str == "Dock" {
                continue;
            }

            // Get PID
            let pid: i32 = msg_send![app, processIdentifier];

            // Create AXUIElement for app
            let app_ax = AXUIElementCreateApplication(pid);

            // Get windows
            let mut windows_ref: CFTypeRef = ptr::null();
            let result =
                AXUIElementCopyAttributeValue(app_ax, cfstring(KAX_WINDOWS), &mut windows_ref);
            if result != 0 || windows_ref.is_null() {
                continue;
            }

            let window_count = CFArrayGetCount(windows_ref as CFArrayRef);
            if window_count == 0 {
                CFRelease(windows_ref);
                continue;
            }

            // Get first window
            let window = CFArrayGetValueAtIndex(windows_ref as CFArrayRef, 0) as AXUIElementRef;

            // Move window to (100, 100)
            let pos = CGPoint { x: 100.0, y: 100.0 };
            let pos_value = AXValueCreate(AXValueType::CGPoint, &pos as *const _ as *const c_void);
            AXUIElementSetAttributeValue(window, cfstring(KAX_POSITION), pos_value);
            CFRelease(pos_value);

            // Resize window to (800, 600)
            let size = CGSize {
                width: 800.0,
                height: 600.0,
            };
            let size_value = AXValueCreate(AXValueType::CGSize, &size as *const _ as *const c_void);
            AXUIElementSetAttributeValue(window, cfstring(KAX_SIZE), size_value);
            CFRelease(size_value);

            CFRelease(windows_ref);

            println!("Moved and resized window for app: {}", rust_str);
            break;
        }

        let _: () = msg_send![pool, drain];
    }
}
