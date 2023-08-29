use std::sync::Arc;
use anyhow::{bail, Result};
use crc::crc32;

pub const ROM_BANK_SIZE: usize = 0x4000;
pub const RAM_BANK_SIZE: usize = 0x2000;

fn is_mbc1_multicart(rom: &[u8]) -> bool {
  if rom.len() != 1048576 {
    return false;
  }
  let nintendo_logo_count = (0..4)
    .map(|page| {
      let start = page * 0x40000 + 0x0104;
      let end = start + 0x30;

      crc32::checksum_ieee(&rom[start..end])
    })
    .filter(|&checksum| checksum == 0x46195417)
    .count();
  nintendo_logo_count >= 3
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
      (self.ram_bank << 4) as usize
    } else {
      (self.ram_bank << 5) as usize
    };
    let lower_bits = if multicart {
      self.rom_bank as usize & 0b1111
    } else {
      self.rom_bank as usize
    };

    let lower_bank = if self.mode { upper_bits } else { 0 };
    let upper_bank = upper_bits | lower_bits;
    (ROM_BANK_SIZE * lower_bank, ROM_BANK_SIZE * upper_bank)
  }
  pub fn ram_offset(&self) -> usize {
    let bank = if self.mode { self.ram_bank as usize } else { 0b00 };
    RAM_BANK_SIZE * bank
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mbc {
  NoMbc { ram: bool, battery: bool },
  Mbc1 { ram: bool, battery: bool, multicart: bool, state: Mbc1State },
}

impl Mbc {
  pub fn new(val: u8, rom: &Arc<[u8]>) -> Result<Self> {
    match val {
      0x00 => Ok(Mbc::NoMbc {
        ram: false,
        battery: false,
      }),
      0x08 => Ok(Mbc::NoMbc {
        ram: true,
        battery: false,
      }),
      0x09 => Ok(Mbc::NoMbc {
        ram: true,
        battery: true,
      }),
      0x01 => Ok(Mbc::Mbc1 {
        ram: false,
        battery: false,
        multicart: is_mbc1_multicart(rom),
        state: Mbc1State::new(),
      }),
      0x02 => Ok(Mbc::Mbc1 {
        ram: true,
        battery: false,
        multicart: is_mbc1_multicart(rom),
        state: Mbc1State::new(),
      }),
      0x03 => Ok(Mbc::Mbc1 {
        ram: true,
        battery: true,
        multicart: is_mbc1_multicart(rom),
        state: Mbc1State::new(),
      }),
      _ => bail!("Invalid cartridge type {}.", val),
    }
  }
  pub fn has_ram(&self) -> bool {
    match *self {
      Mbc::NoMbc { ram, .. } => ram,
      Mbc::Mbc1 { ram, .. } => ram,
    }
  }
}