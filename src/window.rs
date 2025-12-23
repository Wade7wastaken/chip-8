use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use macroquad::prelude::*;

use crate::{Shared, keys::Keys, screen::Screen};

const CONFIG_PANEL_RATIO: f32 = 0.4;

pub async fn window_main(
    screen: Arc<Mutex<Screen>>,
    options: Arc<Mutex<Shared>>,
    keys: Arc<Mutex<Keys>>,
) {
    loop {
        handle_user_input(options.clone(), keys.clone());
        clear_background(BLACK);

        let dx = screen_width() * (1.0 - CONFIG_PANEL_RATIO) / 64.0;
        let dy = screen_height() / 32.0;

        draw_screen(screen.clone(), dx, dy);

        draw_panel(options.clone());

        next_frame().await;
    }
}

fn draw_screen(screen: Arc<Mutex<Screen>>, dx: f32, dy: f32) {
    for (y, row) in screen.lock().unwrap().0.into_iter().enumerate() {
        for i in 0..64 {
            if row & (1 << i) != 0 {
                draw_rectangle(i as f32 * dx, y as f32 * dy, dx, dy, WHITE);
            }
        }
    }
}

fn draw_panel(options: Arc<Mutex<Shared>>) {
    let start_x = screen_width() * (1.0 - CONFIG_PANEL_RATIO);
    let mut y = 30.0;
    let fpx_text = format!("FPS: {:.2}", 1.0 / get_frame_time());
    let size = draw_text(&fpx_text, start_x, y, 20.0, WHITE);
    y += size.height + 10.0;
    let instrs_per_second;
    let instr_count;
    let count_start;
    {
        let options = options.lock().unwrap();
        instrs_per_second = options.instrs_per_second;
        instr_count = options.instr_count;
        count_start = options.count_start;
    }
    let speed_target_text = format!("speed target: {} / sec", instrs_per_second.round());
    let size = draw_text(&speed_target_text, start_x, y, 20.0, WHITE);
    y += size.height + 10.0;

    let instr_speed = instr_count as f64 / (Instant::now() - count_start).as_secs_f64();

    let instr_speed_text = format!("actual speed: {} / sec", instr_speed.round());

    draw_text(&instr_speed_text, start_x, y, 20.0, WHITE);
}

fn handle_user_input(options: Arc<Mutex<Shared>>, keys: Arc<Mutex<Keys>>) {
    let pressed = get_keys_pressed();

    keys.lock().unwrap().set(get_keys_down());

    let mut options = options.lock().unwrap();
    if pressed.contains(&KeyCode::Tab) {
        options.fast_forward = !options.fast_forward;
    }
    if !options.fast_forward {
        if pressed.contains(&KeyCode::Up) {
            options.instrs_per_second += 50.0;
            options.instr_count = 0;
            options.count_start = Instant::now();
        }
        if pressed.contains(&KeyCode::Down) && options.instrs_per_second >= 50.0 {
            options.instrs_per_second -= 50.0;
            options.instr_count = 0;
            options.count_start = Instant::now();
        }
    }
}
