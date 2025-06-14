use crate::geometry::Rect;

pub fn tile_vertical(rects: &mut [Rect], screen: Rect) {
    let n = rects.len();
    if n == 0 {
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
