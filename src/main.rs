use std::{
    fmt,
    hash::Hash,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use memory::Memory;
use registers::Registers;
use screen::Screen;

mod keys;
mod memory;
mod registers;
mod screen;
mod window;

#[macro_export]
macro_rules! tern {
    ($cond:expr, $a:expr, $b:expr) => {
        if $cond { $a } else { $b }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
struct Config {
    bitshift_copies_y: bool,
    jump_with_offset_register: bool,
    update_i_after_store_or_load: bool,
    debug_print_instrs: bool,
}

struct Instr {
    b1: u8,
    b2: u8,
}

impl Instr {
    fn new(b1: u8, b2: u8) -> Self {
        Self { b1, b2 }
    }
    fn as_nibbles(&self) -> (u8, u8, u8, u8) {
        (
            ((self.b1 & 0xf0) >> 4),
            (self.b1 & 0x0f),
            ((self.b2 & 0xf0) >> 4),
            (self.b2 & 0x0f),
        )
    }
    fn as_u8(&self) -> u8 {
        self.b2
    }
    fn as_address(&self) -> usize {
        let a = (self.b1 & 0x0F) as usize;
        a << 8 | (self.b2 as usize)
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02X}{:02X}", self.b1, self.b2)
    }
}

#[derive(Debug, Clone)]
struct Timers {
    delay_timer: u8,
    sound_timer: u8,
}

impl Timers {
    fn new() -> Self {
        Self {
            delay_timer: 0,
            sound_timer: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Shared {
    instrs_per_second: f64,
    fast_forward: bool,
    instr_count: u32,
    count_start: Instant,
}

impl Shared {
    fn reset_instr_count(&mut self) {
        self.instr_count = 0;
        self.count_start = Instant::now();
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            instrs_per_second: 700.0,
            fast_forward: false,
            instr_count: 0,
            count_start: Instant::now(),
        }
    }
}

#[derive(Debug, Clone)]
struct Chip8 {
    config: Config,
    shared: Arc<Mutex<Shared>>,
    memory: Memory,
    pc: usize,
    i: usize,
    stack: Vec<usize>,
    registers: Registers,
    screen: Arc<Mutex<Screen>>,
    timers: Arc<Mutex<Timers>>,
    keys: Arc<Mutex<Keys>>,
}

impl Chip8 {
    fn new(config: Config) -> Self {
        Self {
            config,
            shared: Arc::new(Mutex::new(Shared::default())),
            memory: Memory::new(),
            pc: 0,
            i: 0,
            stack: vec![],
            registers: Registers::new(),
            screen: Arc::new(Mutex::new(Screen::new())),
            timers: Arc::new(Mutex::new(Timers::new())),
            keys: Arc::new(Mutex::new(Keys::default())),
        }
    }

    fn execute_instr(&mut self) {
        {
            let mut shared = self.shared.lock().unwrap();
            shared.instr_count += 1;
            if shared.instr_count > shared.instrs_per_second as u32 {
                shared.reset_instr_count();
            }
        }

        let instr = Instr::new(self.memory.get(self.pc), self.memory.get(self.pc + 1));
        self.pc += 2;

        if self.config.debug_print_instrs {
            println!("running {instr} at address {:#05X}", self.pc);
        }

        match instr.as_nibbles() {
            // Clear screen
            (0x0, 0x0, 0xE, 0x0) => {
                self.screen.lock().unwrap().clear();
            }

            // Return from subroutine
            (0x0, 0x0, 0xE, 0xE) => {
                self.pc = self.stack.pop().unwrap();
            }

            // Execute machine code
            (0x0, _, _, _) => {
                unimplemented!("This instruction executes machine code for a different computer")
            }

            // Jump
            (0x1, _, _, _) => {
                self.pc = instr.as_address();
            }

            // Jump to subroutine
            (0x2, _, _, _) => {
                self.stack.push(self.pc);
                self.pc = instr.as_address();
            }

            // Skip if equal
            (0x3, x, _, _) => {
                if self.registers.get(x) == instr.as_u8() {
                    self.pc += 2;
                }
            }

            // Skip if not equal
            (0x4, x, _, _) => {
                if self.registers.get(x) != instr.as_u8() {
                    self.pc += 2;
                }
            }

            // Skip if registers equal
            (0x5, x, y, 0x0) => {
                if self.registers.get(x) == self.registers.get(y) {
                    self.pc += 2;
                }
            }

            // Set immediate
            (0x6, x, _, _) => {
                self.registers.set(x, instr.as_u8());
            }

            // Add
            (0x7, x, _, _) => {
                *self.registers.get_mut(x) = self.registers.get(x).wrapping_add(instr.as_u8());
            }

            // Copy
            (0x8, x, y, 0x0) => {
                *self.registers.get_mut(x) = self.registers.get(y);
            }

            // Binary OR
            (0x8, x, y, 0x1) => {
                *self.registers.get_mut(x) |= self.registers.get(y);
                self.registers.set(0xF, 0);
            }

            // Binary AND
            (0x8, x, y, 0x2) => {
                *self.registers.get_mut(x) &= self.registers.get(y);
                self.registers.set(0xF, 0);
            }

            // Binary XOR
            (0x8, x, y, 0x3) => {
                *self.registers.get_mut(x) ^= self.registers.get(y);
                self.registers.set(0xF, 0);
            }

            // Add with carry
            (0x8, x, y, 0x4) => {
                let res = self.registers.get(x).overflowing_add(self.registers.get(y));
                self.registers.set(x, res.0);
                self.registers.set(0xF, res.1.into());
            }

            // Subtract with carry
            (0x8, x, y, 0x5) => {
                let res = self.registers.get(x).overflowing_sub(self.registers.get(y));
                self.registers.set(x, res.0);
                self.registers.set(0xF, (!res.1).into());
            }

            // Shift right
            (0x8, x, y, 0x6) => {
                if self.config.bitshift_copies_y {
                    *self.registers.get_mut(y) = self.registers.get(x);
                }
                let n = self.registers.get(x);
                self.registers.set(x, n.wrapping_shr(1));
                self.registers.set(0xF, n & 1);
            }

            // Subtract from with carry
            (0x8, x, y, 0x7) => {
                let res = self.registers.get(y).overflowing_sub(self.registers.get(x));
                self.registers.set(x, res.0);
                self.registers.set(0xF, (!res.1).into());
            }

            // Shift left
            (0x8, x, y, 0xE) => {
                if self.config.bitshift_copies_y {
                    *self.registers.get_mut(y) = self.registers.get(x);
                }
                let n = self.registers.get(x);
                self.registers.set(x, n.wrapping_shl(1));
                self.registers.set(0xF, (n & (1 << 7) != 0).into());
            }

            // Skip if registers not equal
            (0x9, x, y, 0x0) => {
                if self.registers.get(x) != self.registers.get(y) {
                    self.pc += 2;
                }
            }

            // Set index
            (0xA, _, _, _) => self.i = instr.as_address(),

            // Jump with offset
            (0xB, x, _, _) => {
                self.pc = instr.as_address()
                    + self
                        .registers
                        .get(tern!(self.config.jump_with_offset_register, x, 0))
                        as usize;
            }

            // Random
            (0xC, x, _, _) => {
                let r = ::rand::random::<u8>() & instr.as_u8();
                self.registers.set(x, r);
            }

            // Display
            (0xD, x, y, n) => {
                let x = self.registers.get(x) % 64;
                let y = self.registers.get(y) % 32;
                self.registers.set(0xF, 0);

                let mut display = self.screen.lock().unwrap();
                for row in 0..n {
                    if y + row >= 32 {
                        break;
                    }
                    let sprite_data = self.memory.get(self.i + row as usize);
                    for i in 0..8 {
                        if x + i >= 64 {
                            break;
                        }
                        let sprite_pixel = (sprite_data & (1 << (7 - i))) != 0;
                        if sprite_pixel && !display.toggle(x + i, y + row) {
                            self.registers.set(0xF, 1);
                        }
                    }
                }
            }

            // Skip if pressed
            (0xE, x, 0x9, 0xE) => {
                if self.keys.lock().unwrap().get(self.registers.get(x)) {
                    self.pc += 2;
                }
            }
            // Skip if not pressed
            (0xE, x, 0xA, 0x1) => {
                if !self.keys.lock().unwrap().get(self.registers.get(x)) {
                    self.pc += 2;
                }
            }

            // Set delay timer
            (0xF, x, 0x0, 0x7) => {
                let t = self.timers.lock().unwrap();
                self.registers.set(x, t.delay_timer);
            }

            // Get key
            (0xF, x, 0x0, 0xA) => {
                if let Some(idx) = self.keys.lock().unwrap().iter().position(|k| *k) {
                    // key was pressed, store its index in vx
                    self.registers.set(x, idx as u8);
                } else {
                    // no keys pressed
                    self.pc -= 2;
                }
            }

            // Get delay timer
            (0xF, x, 0x1, 0x5) => {
                let mut t = self.timers.lock().unwrap();
                t.delay_timer = self.registers.get(x);
            }

            // Set sound timer
            (0xF, x, 0x1, 0x8) => {
                self.timers.lock().unwrap().sound_timer = self.registers.get(x);
            }

            // Add to index
            (0xF, x, 0x1, 0xE) => {
                self.i += self.registers.get(x) as usize;
                if self.i >= 0x1000 {
                    self.i %= 0x1000;
                    self.registers.set(0xF, 1);
                }
            }

            // Font character
            (0xF, x, 0x2, 0x9) => {
                let ch = self.registers.get(x) & 0x0F;
                self.i = 0x50 + (ch as usize * 5);
            }

            // BCD
            (0xF, x, 0x3, 0x3) => {
                let mut n = self.registers.get(x);
                self.memory.set(self.i, n / 100);
                n %= 100;
                self.memory.set(self.i + 1, n / 10);
                self.memory.set(self.i + 2, n % 10);
            }

            // Store memory
            (0xF, x, 0x5, 0x5) => {
                for dest in 0..=x {
                    self.memory
                        .set(self.i + dest as usize, self.registers.get(dest));
                }
                if self.config.update_i_after_store_or_load {
                    self.i += x as usize + 1;
                }
            }

            // Load memory
            (0xF, x, 0x6, 0x5) => {
                for dest in 0..=x {
                    self.registers
                        .set(dest, self.memory.get(self.i + dest as usize));
                }
                if self.config.update_i_after_store_or_load {
                    self.i += x as usize + 1;
                }
            }

            _ => panic!("unknown instruction {instr}"),
        }
    }

    fn run_at(&mut self, pc: usize) -> ! {
        self.pc = pc;

        let mut frame_delay;
        {
            let options = self.shared.lock().unwrap();
            frame_delay = 1.0 / options.instrs_per_second;
        }

        let mut next_time = Instant::now() + Duration::from_secs_f64(frame_delay);
        loop {
            self.execute_instr();

            let fast_forward;
            {
                let options = self.shared.lock().unwrap();
                fast_forward = options.fast_forward;
                frame_delay = 1.0 / options.instrs_per_second;
            }

            if !fast_forward {
                // while next_time > Instant::now() {}
                thread::sleep(next_time - Instant::now());
                next_time += Duration::from_secs_f64(frame_delay);
            }
        }
    }
}

use window::window_main;

use crate::keys::Keys;

#[macroquad::main("CHIP-8")]
async fn main() {
    let config = Config {
        ..Default::default()
    };
    let mut chip8 = Chip8::new(config);

    let screen = Arc::clone(&chip8.screen);
    let timers = Arc::clone(&chip8.timers);
    let options = Arc::clone(&chip8.shared);
    let keys = Arc::clone(&chip8.keys);

    chip8
        .memory
        .load_bytes_at(0x200, include_bytes!("../programs/games/snake.ch8"));

    thread::Builder::new()
        .name("compute".into())
        .spawn(move || {
            chip8.run_at(0x200);
        })
        .unwrap();

    start_timer_thread(timers);

    window_main(screen, options, keys).await;
}

fn start_timer_thread(timers: Arc<Mutex<Timers>>) {
    thread::spawn(move || {
        let interval = Duration::from_secs_f64(1.0 / 60.0);
        let mut next_time = Instant::now() + interval;
        loop {
            {
                let mut t = timers.lock().unwrap();
                if t.delay_timer != 0 {
                    t.delay_timer -= 1;
                }
                if t.sound_timer != 0 {
                    t.sound_timer -= 1;
                }
            }

            thread::sleep(next_time - Instant::now());
            next_time += interval;
        }
    });
}
