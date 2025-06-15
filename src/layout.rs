use crate::Rect;
use crate::window;
use crate::window::move_and_resize_window;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    Vertical,
    Horizontal,
    Monocle,
}

pub fn tile_vertical(rects: &mut [Rect], screen: Rect) {
    let n = rects.len();
    if n == 0 {
        return;
    }

    if n == 1 {
        rects[0] = screen;
        return;
    }

    let master_ratio = 0.6;
    let master_width = screen.width * master_ratio;
    let stack_width = screen.width - master_width;

    for (i, rect) in rects.iter_mut().enumerate() {
        if i == 0 {
            // Master window
            *rect = Rect {
                x: screen.x,
                y: screen.y,
                width: master_width,
                height: screen.height,
            };
        } else {
            // Stack windows
            let stack_height = screen.height / (n as f64 - 1.0);
            *rect = Rect {
                x: screen.x + master_width,
                y: screen.y + stack_height * (i as f64 - 1.0),
                width: stack_width,
                height: stack_height,
            };
        }
    }
}

pub fn tile_horizontal(rects: &mut [Rect], screen: Rect) {
    let n = rects.len();
    if n == 0 {
        return;
    }

    if n == 1 {
        rects[0] = screen;
        return;
    }

    let master_ratio = 0.6;
    let master_height = screen.height * master_ratio;
    let stack_height = screen.height - master_height;

    for (i, rect) in rects.iter_mut().enumerate() {
        if i == 0 {
            // Master window
            *rect = Rect {
                x: screen.x,
                y: screen.y,
                width: screen.width,
                height: master_height,
            };
        } else {
            // Stack windows
            let stack_width = screen.width / (n as f64 - 1.0);
            *rect = Rect {
                x: screen.x + stack_width * (i as f64 - 1.0),
                y: screen.y + master_height,
                width: stack_width,
                height: stack_height,
            };
        }
    }
}

pub fn tile_monocle(rects: &mut [Rect], screen: Rect) {
    for rect in rects.iter_mut() {
        *rect = screen;
    }
}

pub fn tile_windows(layout: Layout, display: Rect, windows: &[window::Window]) {
    if windows.is_empty() {
        return;
    }

    let mut rects = vec![
        Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0
        };
        windows.len()
    ];

    match layout {
        Layout::Vertical => tile_vertical(&mut rects, display),
        Layout::Horizontal => tile_horizontal(&mut rects, display),
        Layout::Monocle => tile_monocle(&mut rects, display),
    }

    for (window, rect) in windows.iter().zip(rects.iter()) {
        move_and_resize_window(window, *rect);
    }
}
