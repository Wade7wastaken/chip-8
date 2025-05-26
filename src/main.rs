use std::{collections::HashSet, fmt};

use memory::Memory;
use registers::Registers;
use screen::Screen;

mod memory;
mod registers;
mod screen;

const BITSHIFT_COPIES_Y: bool = false;

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
        write!(f, "{:x}{:x}", self.b1, self.b2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Chip8 {
    memory: Memory,
    pc: usize,
    i: usize,
    stack: Vec<usize>,
    registers: Registers,
    display: Screen,
}

impl Chip8 {
    fn new() -> Self {
        Self {
            memory: Memory::new(),
            pc: 0,
            i: 0,
            stack: vec![],
            registers: Registers::new(),
            display: Screen::new(),
        }
    }

    fn run(&mut self) {
        let instr = Instr::new(self.memory.get(self.pc), self.memory.get(self.pc + 1));
        // println!("running {instr} at address {}", self.pc);
        self.pc += 2;
        match instr.as_nibbles() {
            // Clear screen
            (0x0, 0x0, 0xE, 0x0) => {
                self.display.clear();
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
                self.registers
                    .update(x, |x| x.overflowing_add(instr.as_u8()).0);
                // self.registers.get(x) = self.registers.get(x).overflowing_add(instr.as_u8()).0;
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
                if BITSHIFT_COPIES_Y {
                    *self.registers.get_mut(y) = self.registers.get(x);
                }
                let res = self.registers.get(x).overflowing_shr(1);
                self.registers.set(x, res.0);
                self.registers.set(0xF, res.1.into());
            }

            // Subtract from with carry
            (0x8, x, y, 0x7) => {
                let res = self.registers.get(y).overflowing_sub(self.registers.get(x));
                self.registers.set(x, res.0);
                self.registers.set(0xF, (!res.1).into());
            }

            // Shift left
            (0x8, x, y, 0xE) => {
                if BITSHIFT_COPIES_Y {
                    *self.registers.get_mut(y) = self.registers.get(x);
                }
                let res = self.registers.get(x).overflowing_shl(1);
                self.registers.set(x, res.0);
                self.registers.set(0xF, res.1.into());
            }

            // Skip if registers not equal
            (0x9, x, y, 0x0) => {
                if self.registers.get(x) != self.registers.get(y) {
                    self.pc += 2;
                }
            }

            // Set index
            (0xA, _, _, _) => self.i = instr.as_address(),

            // Display
            (0xD, x, y, n) => {
                let x_c = self.registers.get(x) % 64;
                let y_c = self.registers.get(y) % 32;
                self.registers.set(0xF, 0);
                for row in 0..n {
                    let sprite_data = self.memory.get(self.i + row as usize);
                    for i in 0..8 {
                        if (sprite_data & (1 << (7 - i))) != 0
                            && !self
                                .display
                                .toggle((x_c + i) as usize, (y_c + row) as usize)
                        {
                            self.registers.set(0xF, 1);
                        }
                    }
                }
                self.display.show();
            }

            _ => panic!("unknown instruction {instr}"),
        }
    }
}

fn main() {
    let mut chip8 = Chip8::new();
    chip8
        .memory
        .load_bytes_at(0x200, include_bytes!("2-ibm-logo.ch8"));
    chip8.pc = 0x200;

    let mut map = HashSet::new();
    loop {
        if !map.insert(chip8.clone()) {
            break;
        }
        chip8.run();
    }
    chip8.display.show();
    println!("Done!");
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn instr() {
        let instr = Instr::new(0x12, 0x34);
        assert_eq!(instr.as_address(), 0x234);
        assert_eq!(instr.as_u8(), 0x34);
        assert_eq!(instr.as_nibbles(), (0x1, 0x2, 0x3, 0x4));
    }
}
