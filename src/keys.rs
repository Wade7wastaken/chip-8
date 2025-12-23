use std::{collections::HashSet, slice};

use macroquad::input::KeyCode;

#[derive(Debug, Default, Clone)]
pub struct Keys([bool; 0x10]);

const KEY_MAP: [KeyCode; 0x10] = [
    KeyCode::X,    // 0
    KeyCode::Key1, // 1
    KeyCode::Key2, // 2
    KeyCode::Key3, // 3
    KeyCode::Q,    // 4
    KeyCode::W,    // 5
    KeyCode::E,    // 6
    KeyCode::A,    // 7
    KeyCode::S,    // 8
    KeyCode::D,    // 9
    KeyCode::Z,    // A
    KeyCode::C,    // B
    KeyCode::Key4, // C
    KeyCode::R,    // D
    KeyCode::F,    // E
    KeyCode::V,    // F
];

impl Keys {
    pub fn get(&self, x: u8) -> bool {
        self.0[x as usize % 0xF]
    }

    pub fn iter(&self) -> slice::Iter<'_, bool> {
        self.0.iter()
    }

    pub fn set(&mut self, down: HashSet<KeyCode>) {
        for (pressed, key) in self.0.iter_mut().zip(KEY_MAP.iter()) {
            *pressed = down.contains(key);
        }
    }
}
