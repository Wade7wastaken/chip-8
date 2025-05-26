use std::{collections::HashSet, fmt};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Memory([u8; 4096]);

impl Memory {
    fn new() -> Self {
        let mut mem = Memory([0; 4096]);
        mem.set_font();
        mem
    }
    fn get(&self, i: usize) -> u8 {
        self.0[i]
    }
    fn set(&mut self, i: usize, x: u8) {
        self.0[i] = x;
    }
    fn set_font(&mut self) {
        // apparently its common to put the font data here
        self.load_bytes_at(
            0x50,
            &[
                0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
                0x20, 0x60, 0x20, 0x20, 0x70, // 1
                0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
                0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
                0x90, 0x90, 0xF0, 0x10, 0x10, // 4
                0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
                0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
                0xF0, 0x10, 0x20, 0x40, 0x40, // 7
                0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
                0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
                0xF0, 0x90, 0xF0, 0x90, 0x90, // A
                0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
                0xF0, 0x80, 0x80, 0x80, 0xF0, // C
                0xE0, 0x90, 0x90, 0x90, 0xE0, // D
                0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
                0xF0, 0x80, 0xF0, 0x80, 0x80, // F
            ],
        )
    }
    fn load_bytes_at(&mut self, i: usize, data: &[u8]) {
        self.0[i..i + data.len()].clone_from_slice(data);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Display([u64; 32]);

impl Display {
    fn new() -> Self {
        Self([0; 32])
    }
    fn show(&self) {
        println!("{}", "-".repeat(130));
        for row in self.0 {
            print!("|");
            for i in 0..64 {
                if (row & (1 << i)) == 0 {
                    print!("  ");
                } else {
                    print!("██");
                }
            }
            println!("|");
        }
        println!("{}", "-".repeat(130));
    }

    // returns the new state of the pixel
    fn toggle(&mut self, x: usize, y: usize) -> bool {
        self.0[y] |= (1 << x);
        (self.0[y] & (1 << x)) != 0
    }

    // fn write_pixel(&mut self, x: usize, y: usize) {}

    fn clear(&mut self) {
        self.0 = [0; 32];
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Chip8 {
    memory: Memory,
    pc: usize,
    i: usize,
    stack: Vec<u16>,
    registers: [u8; 16],
    display: Display,
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
        write!(f, "{:x}{:x}", self.b1, self.b2)
    }
}

impl Chip8 {
    fn new() -> Self {
        Self {
            memory: Memory::new(),
            pc: 0,
            i: 0,
            stack: vec![],
            registers: [0; 16],
            display: Display::new(),
        }
    }

    fn run(&mut self) {
        let instr = Instr::new(self.memory.get(self.pc), self.memory.get(self.pc + 1));
        // println!("running {instr} at address {}", self.pc);
        self.pc += 2;
        match instr.as_nibbles() {
            (0x0, 0x0, 0xE, 0x0) => {
                self.display.clear();
            }
            (0x1, _, _, _) => {
                self.pc = instr.as_address();
            }
            (0x6, x, _, _) => {
                self.registers[x as usize] = instr.as_u8();
            }
            (0x7, x, _, _) => {
                self.registers[x as usize] =
                    self.registers[x as usize].overflowing_add(instr.as_u8()).0;
            }
            (0xA, _, _, _) => self.i = instr.as_address(),
            (0xD, x, y, n) => {
                let x_c = self.registers[x as usize] % 64;
                let y_c = self.registers[y as usize] % 32;
                self.registers[0xF] = 0;
                for row in 0..n {
                    let sprite_data = self.memory.get(self.i + row as usize);
                    for i in 0..8 {
                        if (sprite_data & (1 << (7 - i))) != 0
                            && !self
                                .display
                                .toggle((x_c + i) as usize, (y_c + row) as usize)
                        {
                            self.registers[0xF] = 1;
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
