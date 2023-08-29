use std::{str, sync::Arc};
use anyhow::{bail, ensure, Result};
use log::info;

use crate::mbc;

#[repr(C)]
pub struct CartridgeHeader {
  entry_point: [u8; 4],
  logo: [u8; 0x0030],
  title: [u8; 0x0010],
  new_licensee_code: [u8; 2],
  sgb_flag: [u8; 1],
  cartridge_type: [u8; 1],
  rom_size: [u8; 1],
  ram_size: [u8; 1],
  destination_code: [u8; 1],
  old_licensee_code: [u8; 1],
  mask_rom_version_number: [u8; 1],
  header_checksum: [u8; 1],
  global_checksum: [u8; 2],
}

impl CartridgeHeader {
  fn rom_banks(&self) -> Result<usize> {
    if self.rom_size[0] <= 0x08 {
      return Ok(1 << (self.rom_size[0] + 1));
    };
    bail!("Invalid rom size {}.", self.rom_size[0]);
  }
  fn ram_size(&self) -> Result<usize> {
    match self.ram_size[0] {
      0x00 => Ok(0),
      0x01 => Ok(2048),
      0x02 => Ok(8192),
      0x03 => Ok(32768),
      0x04 => Ok(131072),
      0x05 => Ok(65536),
      _ => bail!("Invalid ram size {}.", self.ram_size[0]),
    }
  }
}

pub struct Cartridge {
  pub title: String,
  mbc: mbc::Mbc,
  rom_banks: usize,
  rom: Arc<[u8]>,
  rom_offset: (usize, usize),
  pub ram: Box<[u8]>,
  ram_offset: usize,
}

impl Cartridge {
  pub fn new(data: Arc<[u8]>, save: Option<Vec<u8>>) -> Result<Self> {
    ensure!(data.len() >= 0x8000 && data.len() % 0x4000 == 0, "Invalid data size.");
    let header = unsafe {
      std::mem::transmute::<[u8; 0x50], CartridgeHeader>(
        data[0x100..0x150].try_into()?
      )
    };
    let mut chksum: u8 = 0;
    for i in 0x0134..0x014d {
      chksum = chksum.wrapping_sub(data[i]).wrapping_sub(1);
    }
    ensure!(chksum == header.header_checksum[0], "Checksum validation failed.");
    info!("Checksum validation succeeded!");

    let title = str::from_utf8(if header.old_licensee_code[0] == 0x33 {
      &header.title[..11]
    } else {
      &header.title[..15]
    })?.trim_end_matches('\0').to_string();
    let mbc = mbc::Mbc::new(header.cartridge_type[0], &data)?;
    let rom_banks = header.rom_banks()?;
    let rom_size = rom_banks * mbc::ROM_BANK_SIZE;
    let ram_size = header.ram_size()?;

    info!("cartridge info {{ title: {}, type: {}, rom_size: {} B, ram_size: {} B }}",
      title,
      match mbc {
        mbc::Mbc::NoMbc { .. } => "NO MBC",
        mbc::Mbc::Mbc1 { multicart, .. } => if multicart { "MBC1 (multicart)" } else { "MBC1 (not multicart)" },
      },
      rom_size,
      ram_size,
    );

    ensure!(mbc.has_ram() == (ram_size > 0),
      "{:?} cartridge with ram size {} B", mbc, ram_size
    );
    ensure!(data.len() == rom_size,
      "Expected {} bytes of cartridge ROM, got {}", rom_size, data.len()
    );

    let ram = save.unwrap_or(vec![0; ram_size]);
    ensure!(ram.len() == ram_size,
      "Expected {} bytes of save file, got {}", ram_size, ram.len()
    );

    Ok(Cartridge {
      title,
      mbc,
      rom_banks,
      rom: data,
      rom_offset: (0x0000, 0x4000),
      ram: ram.into_boxed_slice(),
      ram_offset: 0x0000,
    })
  }
  pub fn read_0000_3fff(&self, addr: u16) -> u8 {
    self.rom[(self.rom_offset.0 | (addr as usize & 0x3fff)) & (self.rom.len() - 1)]
  }
  pub fn read_4000_7fff(&self, addr: u16) -> u8 {
    self.rom[(self.rom_offset.1 | (addr as usize & 0x3fff)) & (self.rom.len() - 1)]
  }
  pub fn write(&mut self, reladdr: u16, val: u8) {
    match self.mbc {
      mbc::Mbc::NoMbc { .. } => (),
      mbc::Mbc::Mbc1 {
        multicart,
        ref mut state,
        ..
      } => match reladdr >> 8 {
        0x00..=0x1f => {
          state.ram_enable = val & 0x0F == 0x0A;
        }
        0x20..=0x3f => {
          state.rom_bank = if val & 0b11111 == 0b00000 {
            0b00001
          } else {
            val & 0b11111 & (self.rom_banks - 1) as u8
          };
          self.rom_offset = state.rom_offset(multicart);
        }
        0x40..=0x5f => {
          state.ram_bank = val & 0b11;
          self.rom_offset = state.rom_offset(multicart);
          self.ram_offset = state.ram_offset();
        }
        0x60..=0x7f => {
          state.mode = val & 0b1 > 0;
          self.rom_offset = state.rom_offset(multicart);
          self.ram_offset = state.ram_offset();
        }
        _ => (),
      },
    }
  }
  pub fn read_a000_bfff(&self, addr: u16) -> u8 {
    match self.mbc {
      mbc::Mbc::Mbc1 { ref state, .. } if state.ram_enable => self.read_ram(addr, 0xFF),
      _ => 0xFF,
    }
  }
  pub fn write_a000_bfff(&mut self, addr: u16, val: u8) {
    match self.mbc {
      mbc::Mbc::Mbc1 { ref state, .. } if state.ram_enable => self.write_ram(addr, val),
      _ => (),
    }
  }
  fn read_ram(&self, addr: u16, default_val: u8) -> u8 {
    if !self.ram.is_empty() {
      let addr = (self.ram_offset | (addr as usize & 0x1fff)) & (self.ram.len() - 1);
      self.ram[addr]
    } else {
      default_val
    }
  }
  fn write_ram(&mut self, addr: u16, val: u8) {
    if !self.ram.is_empty() {
      let addr = (self.ram_offset | (addr as usize & 0x1fff)) & (self.ram.len() - 1);
      self.ram[addr] = val;
    }
  }
}
