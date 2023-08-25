use crate::cartridge;

pub const ROM_BANK_SIZE: usize = 0x4000;
pub const RAM_BANK_SIZE: usize = 0x2000;

#[derive(Debug, Clone)]
pub struct Mbc1State {
  pub ram_enable: bool,
  pub rom_bank: u8,
  pub ram_bank: u8,
  pub mode: bool,
}

impl Mbc1State {
  fn new() -> Mbc1State {
    Mbc1State {
      ram_enable: false,
      rom_bank: 0b00001,
      ram_bank: 0b00,
      mode: false,
    }
  }
  pub fn rom_offset(&self, multicart: bool) -> (usize, usize) {
    let upper_bits = if multicart {
      self.ram_bank << 4
    } else {
      self.ram_bank << 5
    };
    let lower_bits = if multicart {
      self.rom_bank & 0b1111
    } else {
      self.rom_bank
    };

    let lower_bank = if self.mode { upper_bits as usize } else { 0b00 };
    let upper_bank = (upper_bits | lower_bits) as usize;
    (ROM_BANK_SIZE * lower_bank, ROM_BANK_SIZE * upper_bank)
  }
  pub fn ram_offset(&self) -> usize {
    let bank = if self.mode { self.ram_bank as usize } else { 0b00 };
    RAM_BANK_SIZE * bank
  }
}

pub enum Mbc {
  None,
  Mbc1 { state: Mbc1State, multicart: bool },
}

impl Mbc {
  pub fn new(cartridge_type: &cartridge::CartridgeType) -> Self {
    match cartridge_type {
      cartridge::CartridgeType::NoMbc { .. } => Mbc::None,
      cartridge::CartridgeType::Mbc1 { multicart, .. } => Mbc::Mbc1 {
        state: Mbc1State::new(),
        multicart: *multicart,
      },
    }
  }
}