use crate::Rect;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {}

pub type CGDirectDisplayID = u32;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

unsafe extern "C" {
    pub fn CGMainDisplayID() -> u32;
    pub fn CGDisplayBounds(display: u32) -> CGRect;
    pub fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut u32,
    ) -> i32;
}

pub fn main_screen_rect() -> Rect {
    unsafe {
        let display_id = CGMainDisplayID();
        let bounds = CGDisplayBounds(display_id);
        Rect {
            x: bounds.origin.x,
            y: bounds.origin.y,
            width: bounds.size.width,
            height: bounds.size.height,
        }
    }
}

pub fn all_display_rects() -> Vec<Rect> {
    const MAX_DISPLAYS: usize = 16;
    let mut displays = [0u32; MAX_DISPLAYS];
    let mut count = 0u32;
    let mut rects = Vec::new();

    unsafe {
        if CGGetActiveDisplayList(MAX_DISPLAYS as u32, displays.as_mut_ptr(), &mut count) == 0 {
            for &display_id in &displays[..count as usize] {
                let bounds = CGDisplayBounds(display_id);

                rects.push(Rect {
                    x: bounds.origin.x,
                    y: bounds.origin.y,
                    width: bounds.size.width,
                    height: bounds.size.height,
                });
            }
        }
    }

    rects
}
