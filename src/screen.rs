use crate::tern;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Screen(pub [u64; 32]);

impl Screen {
    pub fn new() -> Self {
        Self([0; 32])
    }
    pub fn show(&self) {
        let border_y = "-".repeat(130);
        println!("{}", border_y);
        for row in self.0 {
            print!("|");
            for i in 0..64 {
                print!("{}", tern!((row & (1 << i)) == 0, "  ", "██"));
            }
            println!("|");
        }
        println!("{}", border_y);
    }

    // returns the new state of the pixel
    pub fn toggle(&mut self, x: usize, y: usize) -> bool {
        self.0[y] |= 1 << x;
        (self.0[y] & (1 << x)) != 0
    }

    pub fn clear(&mut self) {
        self.0 = [0; 32];
    }
}
