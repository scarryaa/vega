use rdev::{Event, EventType, Key, listen};
use std::collections::HashSet;
use std::env;
use std::process::Command;

fn find_vega_executable() -> String {
    let mut path = env::current_exe().expect("Failed to find current exe path");
    path.pop();
    path.push("vega");
    path.to_string_lossy().into_owned()
}

fn main() {
    println!("Listening...");
    let vega_path = find_vega_executable();
    println!("Main program located at: {}", vega_path);

    let mut modifiers = HashSet::new();

    if let Err(error) = listen(move |event: Event| match event.event_type {
        EventType::KeyPress(key) => match key {
            Key::ControlLeft | Key::ControlRight | Key::Alt => {
                modifiers.insert(key);
            }
            Key::KeyT
                if (modifiers.contains(&Key::ControlLeft)
                    || modifiers.contains(&Key::ControlRight))
                    && modifiers.contains(&Key::Alt) =>
            {
                Command::new(&vega_path).arg("cycle").status().ok();
            }
            Key::Return
                if (modifiers.contains(&Key::ControlLeft)
                    || modifiers.contains(&Key::ControlRight))
                    && modifiers.contains(&Key::Alt) =>
            {
                Command::new(&vega_path).arg("promote").status().ok();
            }
            _ => {}
        },
        EventType::KeyRelease(key) => {
            modifiers.remove(&key);
        }
        _ => {}
    }) {
        println!("Error: {:?}", error)
    }
}
