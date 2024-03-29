use std::cmp::max;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct WRam {
  is_cgb: bool,
  svbk: u8,
  ram: Vec<u8>,
}

impl WRam {
  pub fn new(is_cgb: bool) -> Self {
    Self {
      is_cgb,
      svbk: 0,
      ram: vec![0; 0x8000],
    }
  }
  pub fn read(&self, addr: u16) -> u8 {
    if addr == 0xFF70 {
      return self.svbk;
    }
    assert!(addr >= 0xC000 && addr <= 0xFDFF);
    if self.is_cgb {
      if (addr >= 0xC000 && addr <= 0xCFFF) || (addr >= 0xE000 && addr <= 0xEFFF) {
        self.ram[(addr as usize) & 0xfff]
      } else {
        self.ram[max(self.svbk & 7, 1) as usize * 0x1000 + ((addr as usize) & 0xfff)]
      }
    } else {
      self.ram[(addr as usize) & 0x1fff]
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    if addr == 0xFF70 {
      self.svbk = val;
      return;
    }
    assert!(addr >= 0xC000 && addr <= 0xFDFF);
    if self.is_cgb {
      if (addr >= 0xC000 && addr <= 0xCFFF) || (addr >= 0xE000 && addr <= 0xEFFF) {
        self.ram[(addr as usize) & 0xFFF] = val;
      } else {
        self.ram[max(self.svbk & 7, 1) as usize * 0x1000 + ((addr as usize) & 0xFFF)] = val;
      }
    } else {
      self.ram[(addr as usize) & 0x1FFF] = val;
    }
  }
}