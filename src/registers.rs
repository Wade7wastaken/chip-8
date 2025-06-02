#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Registers([u8; 16]);

impl Registers {
    pub fn new() -> Self {
        Self([0; 16])
    }

    pub fn get(&self, x: u8) -> u8 {
        self.0[x as usize]
    }

    pub fn get_mut(&mut self, x: u8) -> &mut u8 {
        &mut self.0[x as usize]
    }

    pub fn set(&mut self, x: u8, v: u8) {
        self.0[x as usize] = v;
    }
}
