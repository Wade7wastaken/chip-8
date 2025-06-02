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
struct Chip8 {
    config: Config,
    memory: Memory,
    pc: usize,
    i: usize,
    stack: Vec<usize>,
    registers: Registers,
    screen: Arc<Mutex<Screen>>,
    timers: Arc<Mutex<Timers>>,
}

impl Chip8 {
    fn new(config: Config) -> Self {
        Self {
            config,
            memory: Memory::new(),
            pc: 0,
            i: 0,
            stack: vec![],
            registers: Registers::new(),
            screen: Arc::new(Mutex::new(Screen::new())),
            timers: Arc::new(Mutex::new(Timers::new())),
        }
    }

    fn execute_instr(&mut self) {
        let instr = Instr::new(self.memory.get(self.pc), self.memory.get(self.pc + 1));
        // println!("running {instr} at address {:#05X}", self.pc);
        self.pc += 2;
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
            (0x8, x, y, 0x0) => *self.registers.get_mut(x) = self.registers.get(y),

            // Binary OR
            (0x8, x, y, 0x1) => *self.registers.get_mut(x) |= self.registers.get(y),

            // Binary AND
            (0x8, x, y, 0x2) => *self.registers.get_mut(x) &= self.registers.get(y),

            // Binary XOR
            (0x8, x, y, 0x3) => *self.registers.get_mut(x) ^= self.registers.get(y),

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

            (0xB, x, _, _) => {
                self.pc = instr.as_address()
                    + self
                        .registers
                        .get(tern!(self.config.jump_with_offset_register, x, 0))
                        as usize;
            }

            (0xC, x, _, _) => {
                self.registers
                    .set(x, ::rand::random::<u8>() & instr.as_u8());
            }

            // Display
            (0xD, x, y, n) => {
                let x_c = self.registers.get(x) % 64;
                let y_c = self.registers.get(y) % 32;
                self.registers.set(0xF, 0);
                let mut display = self.screen.lock().unwrap();
                for row in 0..n {
                    let sprite_data = self.memory.get(self.i + row as usize);
                    for i in 0..8 {
                        if (sprite_data & (1 << (7 - i))) != 0
                            && !display.toggle((x_c + i) as usize, (y_c + row) as usize)
                        {
                            self.registers.set(0xF, 1);
                        }
                    }
                }
                // self.display.show();
            }

            (0xF, x, 0x0, 0x7) => {
                let t = self.timers.lock().unwrap();
                self.registers.set(x, t.delay_timer);
            }

            (0xF, x, 0x1, 0x5) => {
                let mut t = self.timers.lock().unwrap();
                t.delay_timer = self.registers.get(x);
            }

            (0xF, x, 0x1, 0x8) => {
                self.timers.lock().unwrap().sound_timer = self.registers.get(x);
            }

            (0xF, x, 0x1, 0xE) => {
                self.i += self.registers.get(x) as usize;
                if self.i >= 0x1000 {
                    self.i %= 0x1000;
                    self.registers.set(0xF, 1);
                }
            }

            (0xF, x, 0x2, 0x9) => {
                let ch = self.registers.get(x) & 0x0F;
                self.i = 0x50 + (ch as usize * 5);
            }

            (0xF, x, 0x3, 0x3) => {
                let mut n = self.registers.get(x);
                self.memory.set(self.i, n / 100);
                n %= 100;
                self.memory.set(self.i + 1, n / 10);
                self.memory.set(self.i + 2, n % 10);
            }

            (0xF, x, 0x5, 0x5) => {
                for dest in 0..=x {
                    self.memory
                        .set(self.i + dest as usize, self.registers.get(dest));
                }
                if self.config.update_i_after_store_or_load {
                    self.i += x as usize + 1;
                }
            }

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
        loop {
            self.execute_instr();
            // thread::sleep(Duration::from_secs_f64(1.0 / 1000.0));
        }
    }

    // fn run_at_breaking(&mut self, pc: usize) {
    //     self.pc = pc;
    //     let mut map = HashSet::new();
    //     loop {
    //         if !map.insert(self.clone()) {
    //             break;
    //         }
    //         self.execute_instr();
    //     }
    // }
}

use macroquad::prelude::*;
use window::draw;

#[macroquad::main("CHIP-8")]
async fn main() {
    let mut chip8 = Chip8::new(Config::default());

    let screen = Arc::clone(&chip8.screen);
    let timers = Arc::clone(&chip8.timers);

    chip8
        .memory
        .load_bytes_at(0x200, include_bytes!("timer.ch8"));

    thread::spawn(move || {
        chip8.run_at(0x200);
    });

    start_timer_thread(timers);

    draw(screen).await;
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

// #[cfg(test)]
// mod tests {
//     use crate::*;

//     #[test]
//     fn chip8_logo() {
//         let mut chip8 = Chip8::new(Config::default());
//         chip8
//             .memory
//             .load_bytes_at(0x200, include_bytes!("1-chip8-logo.ch8"));

//         chip8.run_at_breaking(0x200);
//         let expected = [
//             0,
//             3378249476730880,
//             2694781007314944,
//             90798382399488,
//             161167525036032,
//             301904073867264,
//             242874915766272,
//             0,
//             0,
//             35747455993182208,
//             64035830873586688,
//             54536273747873280,
//             54536204088772352,
//             54536204894119680,
//             28007188487997184,
//             17749844529321728,
//             32527349553358592,
//             63543249714546432,
//             54289622416230144,
//             54289502157145856,
//             54305067118626560,
//             63587212999888384,
//             35965178807712768,
//             17889757402822656,
//             0,
//             0,
//             1710891725545472,
//             2657666108375040,
//             4245369091145728,
//             304736603619328,
//             4052551168966656,
//             0,
//         ];
//         assert_eq!(chip8.display.0, expected);
//     }

//     #[test]
//     fn ibm_logo() {
//         let mut chip8 = Chip8::new(Config::default());
//         chip8
//             .memory
//             .load_bytes_at(0x200, include_bytes!("2-ibm-logo.ch8"));

//         chip8.run_at_breaking(0x200);
//         let expected = [
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//             94435122047086592,
//             90071992547409920,
//             40462573361950720,
//             0,
//             91163777051115520,
//             126100789566373888,
//             73179062604120064,
//             72057594037927936,
//             1055222990618624,
//             36028797018963968,
//             1019491614179328,
//             126100789566373888,
//             76436119921618944,
//             54043195528445952,
//             130468317112561664,
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//             0,
//         ];
//         assert_eq!(chip8.display.0, expected);
//     }

//     #[test]
//     fn corax() {
//         let mut chip8 = Chip8::new(Config::default());
//         chip8
//             .memory
//             .load_bytes_at(0x200, include_bytes!("3-corax+.ch8"));

//         chip8.run_at_breaking(0x200);
//         let expected = [
//             0,
//             133983583585698140,
//             2937518882502486168,
//             1804845124783184208,
//             631639863759472988,
//             0,
//             133984133349900628,
//             2991560978523760796,
//             1804845124766931280,
//             703697182927882576,
//             0,
//             133984133349900636,
//             2937518057882134668,
//             1750801379499448656,
//             703696908049975628,
//             0,
//             90073762088354140,
//             2924006434353719440,
//             1825112147728013640,
//             668795385327389000,
//             0,
//             2044435628380,
//             44926047627420,
//             26749472872784,
//             10840662870348,
//             0,
//             8160524294353060172,
//             4743460580748961928,
//             3536479971777452360,
//             8433001039834909020,
//             0,
//             0,
//         ];
//         assert_eq!(chip8.display.0, expected);
//     }

//     #[test]
//     fn flags() {
//         let mut chip8 = Chip8::new(Config::default());
//         chip8
//             .memory
//             .load_bytes_at(0x200, include_bytes!("4-flags.ch8"));

//         chip8.run_at_breaking(0x200);
//         let expected = [
//             123145315234597,
//             768497238380205399,
//             461108898343039861,
//             153808519257264469,
//             0,
//             123145323282439,
//             12297697441062365862,
//             7378657167445878372,
//             2459581709469884967,
//             0,
//             123145331671047,
//             768482394981313185,
//             461075363246663271,
//             153809069000368679,
//             0,
//             0,
//             123145323623207,
//             12297697441062671697,
//             7378657167445996401,
//             2459581709470029143,
//             0,
//             123145331671047,
//             768482394981313185,
//             461075363246663271,
//             153809069000368679,
//             0,
//             0,
//             8160522525294687607,
//             4743416490269947685,
//             3536451716994241829,
//             8432990339232789799,
//             0,
//         ];
//         assert_eq!(chip8.display.0, expected);
//     }

//     #[test]
//     fn instr() {
//         let instr = Instr::new(0x12, 0x34);
//         assert_eq!(instr.as_address(), 0x234);
//         assert_eq!(instr.as_u8(), 0x34);
//         assert_eq!(instr.as_nibbles(), (0x1, 0x2, 0x3, 0x4));
//     }
// }
