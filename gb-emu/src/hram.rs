use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct HRam(Vec<u8>);

impl HRam {
  pub fn new() -> Self {
    Self(vec![0; 0x80])
  }
  pub fn read(&self, addr: u16) -> u8 {
    self.0[(addr as usize) & 0x7f]
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    self.0[(addr as usize) & 0x7f] = val;
  }
}