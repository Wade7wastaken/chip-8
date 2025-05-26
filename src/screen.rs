#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Screen([u64; 32]);

impl Screen {
    pub fn new() -> Self {
        Self([0; 32])
    }
    pub fn show(&self) {
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
    pub fn toggle(&mut self, x: usize, y: usize) -> bool {
        self.0[y] |= 1 << x;
        (self.0[y] & (1 << x)) != 0
    }

    // fn write_pixel(&mut self, x: usize, y: usize) {}

    pub fn clear(&mut self) {
        self.0 = [0; 32];
    }
}
