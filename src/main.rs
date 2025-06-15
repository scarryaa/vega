use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::{
    layout::{Layout, tile_windows},
    window::{
        CFRelease, Window, collect_windows, get_focused_window_ref, is_window_minimized,
        window_rect,
    },
};

mod core_graphics;
mod geometry;
mod layout;
mod window;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {}

type WindowSignature = (String, String);

#[derive(Serialize, Deserialize, Debug)]
struct State {
    current_layout: Layout,
    window_order: Vec<WindowSignature>,
}

impl Default for State {
    fn default() -> Self {
        State {
            current_layout: Layout::Vertical,
            window_order: Vec::new(),
        }
    }
}

fn get_state_file_path() -> PathBuf {
    let mut path = env::temp_dir();
    path.push("vega_state.json");
    path
}

fn load_state() -> State {
    let path = get_state_file_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_state(state: &State) {
    let path = get_state_file_path();
    if let Ok(content) = serde_json::to_string_pretty(state) {
        fs::write(path, content).ok();
    }
}

fn retile_windows(layout: Layout, windows: &[Window]) {
    let main_display = core_graphics::main_screen_rect();

    let filtered_windows: Vec<_> = windows
        .iter()
        .filter(|w| {
            if is_window_minimized(w) {
                return false;
            }
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
        .cloned()
        .collect();

    println!(
        "Tiling {} windows using {:?} layout",
        filtered_windows.len(),
        layout
    );

    tile_windows(layout, main_display, &filtered_windows);
}

impl From<crate::window::AXUIElementRef> for crate::window::SendableAXUIElementRef {
    fn from(ax_ref: crate::window::AXUIElementRef) -> Self {
        Self(ax_ref)
    }
}

impl Clone for Window {
    fn clone(&self) -> Self {
        Window {
            ax_ref: unsafe { crate::window::CFRetain(*self.ax_ref) }.into(),
            app_name: self.app_name.clone(),
            title: self.title.clone(),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vega <cycle|promote>");
        return;
    }

    let command = &args[1];

    let mut state = load_state();
    let all_layouts = [Layout::Vertical, Layout::Horizontal, Layout::Monocle];

    let live_windows = collect_windows();
    let mut live_map: HashMap<WindowSignature, Window> = live_windows
        .into_iter()
        .map(|w| ((w.app_name.clone(), w.title.clone()), w))
        .collect();

    let mut ordered_windows: Vec<Window> = state
        .window_order
        .iter()
        .filter_map(|sig| live_map.remove(sig))
        .collect();

    ordered_windows.extend(live_map.into_values());

    match command.as_str() {
        "cycle" => {
            let current_index = all_layouts
                .iter()
                .position(|&l| l == state.current_layout)
                .unwrap_or(0);

            let next_index = (current_index + 1) % all_layouts.len();
            state.current_layout = all_layouts[next_index];
            println!("Switching to layout: {:?}", state.current_layout);

            retile_windows(state.current_layout, &ordered_windows);
        }
        "promote" => {
            if let Some(focused_ref) = get_focused_window_ref() {
                if let Some(pos) = ordered_windows
                    .iter()
                    .position(|w| unsafe { crate::window::CFEqual(*w.ax_ref, *focused_ref) } != 0)
                {
                    let master_window = ordered_windows.remove(pos);
                    println!("Promoting '{}'", master_window.app_name);
                    ordered_windows.insert(0, master_window);

                    retile_windows(state.current_layout, &ordered_windows);
                }

                unsafe { CFRelease(*focused_ref) };
            } else {
                println!("Could not find a focused window");
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
        }
    }

    state.window_order = ordered_windows
        .iter()
        .map(|w| (w.app_name.clone(), w.title.clone()))
        .collect();
    save_state(&state);
}
