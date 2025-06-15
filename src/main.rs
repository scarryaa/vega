use rdev::{Event, EventType, Key, listen};

use crate::{
    geometry::Rect,
    layout::{Layout, tile_windows},
    window::{collect_windows, is_window_minimized, window_rect},
};
use std::{collections::HashSet, thread};
use std::{sync::mpsc, time::Duration};

mod core_graphics;
mod geometry;
mod layout;
mod window;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {}

fn retile_windows(layout: Layout) {
    let main_display = core_graphics::main_screen_rect();
    let windows = collect_windows();

    let filtered_windows: Vec<_> = windows
        .into_iter()
        .filter(|w| {
            // Don't tile minimized windows
            if is_window_minimized(w) {
                return false;
            }

            // Filter for windows on main display
            if let Some(rect) = window_rect(w) {
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

    println!("Tiling {} windows", filtered_windows.len());
    tile_windows(layout, main_display, &filtered_windows);
}

fn main() {
    let mut layout = Layout::Vertical;

    // Initial layout
    retile_windows(layout);

    let (tx, rx) = mpsc::channel();

    // Hotkey thread
    std::thread::spawn(move || {
        let mut modifiers = HashSet::new();
        if let Err(error) = listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    if key == Key::ControlLeft || key == Key::ControlRight || key == Key::Alt {
                        modifiers.insert(key);
                    }
                    // Check for Control + Option + T
                    if key == Key::KeyT
                        && (modifiers.contains(&Key::ControlLeft)
                            || modifiers.contains(&Key::ControlRight))
                        && (modifiers.contains(&Key::Alt))
                    {
                        tx.send(()).ok();
                    }
                }
                EventType::KeyRelease(key) => {
                    modifiers.remove(&key);
                }
                _ => {}
            }
        }) {
            println!("Error: {:?}", error);
        }
    });

    loop {
        // Check for hotkey event
        if let Ok(()) = rx.try_recv() {
            layout = match layout {
                Layout::Vertical => Layout::Horizontal,
                Layout::Horizontal => Layout::Vertical,
            };
            println!("Switched layout to {:?}", layout);

            retile_windows(layout);
        }

        thread::sleep(Duration::from_millis(100));
    }
}
