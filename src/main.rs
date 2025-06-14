use crate::{
    geometry::Rect,
    layout::tile_vertical,
    window::{collect_windows, move_and_resize_window},
};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

mod core_graphics;
mod geometry;
mod layout;
mod window;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {}

fn main() {
    unsafe {
        let pool: *mut AnyObject = msg_send![class!(NSAutoreleasePool), new];

        let main_display = core_graphics::main_screen_rect();
        let windows = collect_windows();

        let filtered_windows: Vec<_> = windows
            .into_iter()
            .filter(|w| {
                if let Some(rect) = window::window_rect(w) {
                    let cx = rect.x + rect.width / 2.0;
                    let cy = rect.y + rect.height / 2.0;
                    cx >= main_display.x
                        && cx < main_display.x + main_display.width
                        && cy >= main_display.y
                        && cy < main_display.y + main_display.height
                } else {
                    false
                }
            })
            .collect();

        let mut rects = vec![
            Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0
            };
            filtered_windows.len()
        ];
        tile_vertical(&mut rects, main_display);

        for (window, rect) in filtered_windows.iter().zip(rects.iter()) {
            move_and_resize_window(window, *rect);
            println!("Moved window for app: {}", window.app_name);
        }

        let _: () = msg_send![pool, drain];
    }
}
