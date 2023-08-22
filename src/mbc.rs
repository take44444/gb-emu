use crate::cartridge;

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

#[derive(Debug, Clone)]
pub struct Mbc1State {
  pub ramg: bool,
  pub bank1: u8,
  pub bank2: u8,
  pub mode: bool,
}

impl Mbc1State {
  fn new() -> Mbc1State {
    Mbc1State {
      ramg: false,
      bank1: 0b0_0001,
      bank2: 0b00,
      mode: false,
    }
  }
  pub fn rom_offset(&self, multicart: bool) -> (usize, usize) {
    let upper_bits = if multicart {
      self.bank2 << 4
    } else {
      self.bank2 << 5
    };
    let lower_bits = if multicart {
      self.bank1 & 0b1111
    } else {
      self.bank1
    };

    let lower_bank = if self.mode { upper_bits as usize } else { 0b00 };
    let upper_bank = (upper_bits | lower_bits) as usize;
    (ROM_BANK_SIZE * lower_bank, ROM_BANK_SIZE * upper_bank)
  }
  pub fn ram_offset(&self) -> usize {
    let bank = if self.mode { self.bank2 as usize } else { 0b00 };
    RAM_BANK_SIZE * bank
  }
}

pub enum Mbc {
  None,
  Mbc1 { state: Mbc1State, multicart: bool },
  // Mbc2,
  // Mbc3,
  // Mbc5,
  // Huc1,
}

impl Mbc {
  pub fn new(cartridge_type: &cartridge::CartridgeType) -> Self {
    match cartridge_type {
      cartridge::CartridgeType::NoMbc { .. } => Mbc::None,
      cartridge::CartridgeType::Mbc1 { multicart, .. } => Mbc::Mbc1 {
        state: Mbc1State::new(),
        multicart: *multicart,
      },
      _ => panic!("Unsupported cartridge type {:?}", cartridge_type),
    }
  }
}