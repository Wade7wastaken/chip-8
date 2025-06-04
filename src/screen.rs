#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Screen(pub [u64; 32]);

impl Screen {
    pub fn new() -> Self {
        Self([0; 32])
    }

    // returns the new state of the pixel
    pub fn toggle(&mut self, x: usize, y: usize) -> bool {
        self.0[y] ^= 1 << x;
        (self.0[y] & (1 << x)) != 0
    }

    pub fn clear(&mut self) {
        self.0 = [0; 32];
    }
}
