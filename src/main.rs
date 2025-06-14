use rdev::{Event, EventType, Key, listen};

use crate::{
    geometry::Rect,
    layout::{Layout, tile_windows},
    window::{collect_windows, window_rect},
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

fn main() {
    let main_display = core_graphics::main_screen_rect();
    let windows = collect_windows();
    let filtered_windows: Vec<_> = windows
        .into_iter()
        .filter(|w| {
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

    let display = main_display;
    let windows = filtered_windows;

    // Initial layout
    let mut layout = Layout::Vertical;
    tile_windows(layout, display, &windows);

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
            println!("Switched layout!");

            tile_windows(layout, display, &windows);
        }

        thread::sleep(Duration::from_millis(100));
    }
}
