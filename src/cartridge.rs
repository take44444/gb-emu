use std::sync::Arc;
use anyhow::{Result, bail, ensure};
use log::info;
use crc::crc32;

use crate::mbc;

fn rom_banks(val: u8) -> Result<usize> {
  if val <= 0x08 {
    return Ok(1 << (val + 1));
  };
  bail!("Invalid rom size {}.", val);
}

fn rom_size(val: u8) -> Result<usize> {
  Ok(rom_banks(val)? * mbc::ROM_BANK_SIZE)
}

fn ram_size(val: u8) -> Result<usize> {
  match val {
    0x00 => Ok(0),
    0x01 => Ok(2048),
    0x02 => Ok(8192),
    0x03 => Ok(32768),
    0x04 => Ok(131072),
    0x05 => Ok(65536),
    _ => bail!("Invalid ram size {}.", val),
  }
}

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

pub struct Cartridge {
  mbc: mbc::Mbc,
  rom: Arc<[u8]>,
  rom_offset: (usize, usize),
  ram: Box<[u8]>,
  ram_offset: usize,
}

impl Cartridge {
  pub fn new(data: Arc<[u8]>) -> Result<Self> {
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

    let mut cartridge_type = CartridgeType::new(header.cartridge_type[0])?;
    if let CartridgeType::Mbc1 { multicart, .. } = &mut cartridge_type {
      *multicart = is_mbc1_multicart(&data);
    }
    let rom_size = rom_size(header.rom_size[0])?;
    let ram_size = ram_size(header.ram_size[0])?;

    info!("cartridge info {{ title: {}, type: {}, rom_size: {} B, ram_size: {} B }}",
      String::from_utf8_lossy(if header.old_licensee_code[0] == 0x33 {
        &header.title[..11]
      } else {
        &header.title[..15]
      }).trim_end_matches('\0'),
      match cartridge_type {
        CartridgeType::NoMbc { .. } => "NO MBC",
        CartridgeType::Mbc1 { multicart, .. } => if multicart { "MBC1 (multicart)" } else { "MBC1 (not multicart)" },
      },
      rom_size,
      ram_size,
    );

    ensure!(cartridge_type.has_ram_chip() == (ram_size > 0),
      "{:?} cartridge with ram size {} B", cartridge_type, ram_size
    );
    ensure!(data.len() == rom_size,
      "Expected {} bytes of cartridge ROM, got {}", rom_size, data.len()
    );

    Ok(Cartridge {
      mbc: mbc::Mbc::new(&cartridge_type),
      rom: data,
      rom_offset: (0x0000, 0x4000),
      ram: vec![0; ram_size].into_boxed_slice(),
      ram_offset: 0x0000,
    })
  }
  pub fn read_0000_3fff(&self, addr: u16) -> u8 {
    let (rom_lower, _) = self.rom_offset;
    self.rom[(rom_lower | (addr as usize & 0x3fff)) & (self.rom.len() - 1)]
  }
  pub fn read_4000_7fff(&self, addr: u16) -> u8 {
    let (_, rom_upper) = self.rom_offset;
    self.rom[(rom_upper | (addr as usize & 0x3fff)) & (self.rom.len() - 1)]
  }
  pub fn write(&mut self, reladdr: u16, val: u8) {
    match self.mbc {
      mbc::Mbc::None => (),
      mbc::Mbc::Mbc1 {
        ref mut state,
        multicart,
      } => match reladdr >> 8 {
        0x00..=0x1f => {
          state.ram_enable = val & 0x0F == 0x0A;
        }
        0x20..=0x3f => {
          state.rom_bank = if val & 0b11111 == 0b00000 {
            0b00001
          } else {
            val & 0b11111
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CartridgeType {
  NoMbc {
    ram: bool,
    battery: bool,
  },
  Mbc1 {
    ram: bool,
    battery: bool,
    multicart: bool,
  },
}

impl CartridgeType {
  fn new(val: u8) -> Result<CartridgeType> {
    match val {
      0x00 => Ok(CartridgeType::NoMbc {
        ram: false,
        battery: false,
      }),
      0x08 => Ok(CartridgeType::NoMbc {
        ram: true,
        battery: false,
      }),
      0x09 => Ok(CartridgeType::NoMbc {
        ram: true,
        battery: true,
      }),
      0x01 => Ok(CartridgeType::Mbc1 {
        ram: false,
        battery: false,
        multicart: false,
      }),
      0x02 => Ok(CartridgeType::Mbc1 {
        ram: true,
        battery: false,
        multicart: false,
      }),
      0x03 => Ok(CartridgeType::Mbc1 {
        ram: true,
        battery: true,
        multicart: false,
      }),
      _ => bail!("Invalid cartridge type {}.", val),
    }
  }
  fn has_ram_chip(&self) -> bool {
    match *self {
      CartridgeType::NoMbc { ram, .. } => ram,
      CartridgeType::Mbc1 { ram, .. } => ram,
    }
  }
}
