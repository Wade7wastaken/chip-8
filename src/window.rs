use std::sync::{Arc, Mutex};

use macroquad::prelude::*;

use crate::screen::Screen;

const CONFIG_PANEL_RATIO: f32 = 0.2;

pub async fn draw(screen: Arc<Mutex<Screen>>) {
    loop {
        clear_background(BLACK);
        // draw_fps();
        // get_frame_time();

        let dx = screen_width() * (1.0 - CONFIG_PANEL_RATIO) / 64.0;
        let dy = screen_height() / 32.0;

        for (y, row) in screen.lock().unwrap().0.into_iter().enumerate() {
            for i in 0..64 {
                if row & (1 << i) != 0 {
                    draw_rectangle(i as f32 * dx, y as f32 * dy, dx, dy, WHITE);
                }
            }
        }

        draw_panel();

        next_frame().await;
    }
}

fn draw_panel() {
    let start_x = screen_width() * (1.0 - CONFIG_PANEL_RATIO);
    let mut y = 30.0;
    let size = draw_text(
        &format!("FPS: {}", 1.0 / get_frame_time()),
        start_x,
        y,
        20.0,
        WHITE,
    );
    y += size.height;
    draw_text("ABC", start_x, y, 20.0, WHITE);
}
