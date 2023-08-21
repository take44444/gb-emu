#[derive(Clone)]
pub struct HRam {
  ram: Box<[u8; 0x80]>,
}

impl HRam {
  pub fn new() -> HRam {
    HRam {
      ram: Box::new([0; 0x80]),
    }
  }
  pub fn read(&self, addr: u16) -> u8 {
    self.ram[(addr as usize) & 0x7f]
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    self.ram[(addr as usize) & 0x7f] = val;
  }
}