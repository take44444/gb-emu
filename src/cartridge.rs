use std::{fmt, sync::Arc};
use anyhow::{Result, bail, ensure};
use log::info;
use crc::crc32;

use crate::{mbc, cartridge};

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

fn is_mbc1_multicart(rom: &[u8]) -> bool {
  if rom.len() != 1_048_576 {
    return false;
  }

  let nintendo_logo_count = (0..4)
    .map(|page| {
      let start = page * 0x40000 + 0x0104;
      let end = start + 0x30;

      crc32::checksum_ieee(&rom[start..end])
    })
    .filter(|&checksum| checksum == 0x4619_5417)
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

impl fmt::Debug for CartridgeHeader {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("CartridgeHeader")
      .field("entry_point", &format!("{:02X?}", self.entry_point))
      .field("logo", &format!("{:02X?}", self.logo))
      .field("title", &String::from_utf8_lossy(&self.title))
      .field("new_licensee_code", &format!("{:02X?}", self.new_licensee_code))
      .field("sgb_flag", &format!("{:02X?}", self.sgb_flag))
      .field("cartridge_type", &format!("{:02X?}", self.cartridge_type))
      .field("rom_size", &format!("{:02X?}", self.rom_size))
      .field("ram_size", &format!("{:02X?}", self.ram_size))
      .field("destination_code", &format!("{:02X?}", self.destination_code))
      .field("old_licensee_code", &format!("{:02X?}", self.old_licensee_code))
      .field("mask_rom_version_number", &format!("{:02X?}", self.mask_rom_version_number))
      .field("header_checksum", &format!("{:02X?}", self.header_checksum))
      .field("global_checksum", &format!("{:02X?}", self.global_checksum))
      .finish()
  }
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
    info!("{:?}", header);
    let mut chksum: u8 = 0;
    for i in 0x0134..0x014d {
      chksum = chksum.wrapping_sub(data[i]).wrapping_sub(1);
    }
    ensure!(chksum == header.header_checksum[0], "Checksum validation failed.");
    info!("Checksum validation succeeded!");

    let mut cartridge_type = CartridgeType::from_u8(header.cartridge_type[0])?;
    if let CartridgeType::Mbc1 { multicart, .. } = &mut cartridge_type {
      *multicart = is_mbc1_multicart(&data);
    }
    let rom_size = CartridgeRomSize::from_u8(header.rom_size[0])?;
    let ram_size = CartridgeRamSize::from_u8(header.ram_size[0])?;
    ensure!(!cartridge_type.has_ram_chip() || ram_size != CartridgeRamSize::NoRam,
      "{:?} cartridge without ram", cartridge_type
    );
    ensure!(cartridge_type.has_ram_chip() || ram_size == CartridgeRamSize::NoRam,
      "{:?} cartridge with ram size {:02x}", cartridge_type, header.ram_size[0]
    );
    ensure!(data.len() == rom_size.as_usize(),
      "Expected {} bytes of cartridge ROM, got {:?}", rom_size.as_usize(), data.len()
    );

    let mbc = mbc::Mbc::new(&cartridge_type)?;
    Ok(Cartridge {
      mbc: mbc,
      rom: data,
      rom_offset: (0x0000, 0x4000),
      ram: vec![0; ram_size.as_usize()].into_boxed_slice(),
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
          state.ram_enable = val & 0b1111 == 0b1010;
        }
        0x20..=0x3f => {
          state.bank1 = if val & 0b1_1111 == 0b0_0000 {
            0b0_0001
          } else {
            val & 0b1_1111
          };
          self.rom_offset = state.rom_offset(multicart);
        }
        0x40..=0x5f => {
          state.bank2 = val & 0b11;
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
  pub fn read_a000_bfff(&self, addr: u16, default_val: u8) -> u8 {
    match self.mbc {
      mbc::Mbc::Mbc1 { ref state, .. } if state.ram_enable => self.read_ram(addr, default_val),
      _ => default_val,
    }
  }
  pub fn write_a000_bfff(&mut self, addr: u16, val: u8) {
    match self.mbc {
      mbc::Mbc::Mbc1 { ref state, .. } if state.ram_enable => self.write_ram(addr, val),
      _ => (),
    }
  }
  fn ram_addr(&self, addr: u16) -> usize {
    (self.ram_offset | (addr as usize & 0x1fff)) & (self.ram.len() - 1)
  }
  fn read_ram(&self, addr: u16, default_val: u8) -> u8 {
    if !self.ram.is_empty() {
      let addr = self.ram_addr(addr);
      self.ram[addr]
    } else {
      default_val
    }
  }
  fn write_ram(&mut self, addr: u16, val: u8) {
    if !self.ram.is_empty() {
      let addr = self.ram_addr(addr);
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
  Mbc2 {
    battery: bool,
  },
  Mbc3 {
    ram: bool,
    battery: bool,
    rtc: bool,
  },
  Mbc5 {
    ram: bool,
    battery: bool,
    rumble: bool,
  },
  Mbc6,
  Mbc7,
  Huc1,
  Huc3,
}

impl CartridgeType {
  fn from_u8(val: u8) -> Result<CartridgeType> {
    use self::CartridgeType::*;
    match val {
      0x00 => Ok(NoMbc {
        ram: false,
        battery: false,
      }),
      0x08 => Ok(NoMbc {
        ram: true,
        battery: false,
      }),
      0x09 => Ok(NoMbc {
        ram: true,
        battery: true,
      }),
      0x01 => Ok(Mbc1 {
        ram: false,
        battery: false,
        multicart: false,
      }),
      0x02 => Ok(Mbc1 {
        ram: true,
        battery: false,
        multicart: false,
      }),
      0x03 => Ok(Mbc1 {
        ram: true,
        battery: true,
        multicart: false,
      }),
      0x05 => Ok(Mbc2 { battery: false }),
      0x06 => Ok(Mbc2 { battery: true }),
      0x11 => Ok(Mbc3 {
        ram: false,
        battery: false,
        rtc: false,
      }),
      0x12 => Ok(Mbc3 {
        ram: true,
        battery: false,
        rtc: false,
      }),
      0x13 => Ok(Mbc3 {
        ram: true,
        battery: true,
        rtc: false,
      }),
      0x0f => Ok(Mbc3 {
        ram: false,
        battery: true,
        rtc: true,
      }),
      0x10 => Ok(Mbc3 {
        ram: true,
        battery: true,
        rtc: true,
      }),
      0x19 => Ok(Mbc5 {
        ram: false,
        battery: false,
        rumble: false,
      }),
      0x1a => Ok(Mbc5 {
        ram: true,
        battery: false,
        rumble: false,
      }),
      0x1b => Ok(Mbc5 {
        ram: true,
        battery: true,
        rumble: false,
      }),
      0x1c => Ok(Mbc5 {
        ram: false,
        battery: false,
        rumble: true,
      }),
      0x1d => Ok(Mbc5 {
        ram: true,
        battery: false,
        rumble: true,
      }),
      0x1e => Ok(Mbc5 {
        ram: true,
        battery: true,
        rumble: true,
      }),
      0x20 => Ok(Mbc6),
      0x22 => Ok(Mbc7),
      0xff => Ok(Huc1),
      0xfe => Ok(Huc3),
      _ => bail!("Invalid cartridge type {}.", val),
    }
  }
  fn has_ram_chip(&self) -> bool {
    use self::CartridgeType::*;
    match *self {
      NoMbc { ram, .. } => ram,
      Mbc1 { ram, .. } => ram,
      Mbc2 { .. } => false, // MBC2 has internal RAM and doesn't use a RAM chip
      Mbc3 { ram, .. } => ram,
      Mbc5 { ram, .. } => ram,
      Mbc6 | Mbc7 | Huc1 | Huc3 => true,
    }
  }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CartridgeRomSize {
  NoRomBanks = 0x00,
  RomBanks4 = 0x01,
  RomBanks8 = 0x02,
  RomBanks16 = 0x03,
  RomBanks32 = 0x04,
  RomBanks64 = 0x05,
  RomBanks128 = 0x06,
  RomBanks256 = 0x07,
  RomBanks512 = 0x08,
}

impl fmt::Debug for CartridgeRomSize {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    use self::CartridgeRomSize::*;
    write!(
      f,
      "{}",
      match *self {
        NoRomBanks => "256 kbit",
        RomBanks4 => "512 kbit",
        RomBanks8 => "1 Mbit",
        RomBanks16 => "2 Mbit",
        RomBanks32 => "4 Mbit",
        RomBanks64 => "8 Mbit",
        RomBanks128 => "16 Mbit",
        RomBanks256 => "32 Mbit",
        RomBanks512 => "64 Mbit",
      }
    )
  }
}

impl CartridgeRomSize {
  fn from_u8(val: u8) -> Result<CartridgeRomSize> {
    use self::CartridgeRomSize::*;
    match val {
      0x00 => Ok(NoRomBanks),
      0x01 => Ok(RomBanks4),
      0x02 => Ok(RomBanks8),
      0x03 => Ok(RomBanks16),
      0x04 => Ok(RomBanks32),
      0x05 => Ok(RomBanks64),
      0x06 => Ok(RomBanks128),
      0x07 => Ok(RomBanks256),
      0x08 => Ok(RomBanks512),
      _ => bail!("Invalid rom size {}.", val),
    }
  }
  pub fn banks(&self) -> usize {
    use self::CartridgeRomSize::*;
    match *self {
      NoRomBanks => 2,
      RomBanks4 => 4,
      RomBanks8 => 8,
      RomBanks16 => 16,
      RomBanks32 => 32,
      RomBanks64 => 64,
      RomBanks128 => 128,
      RomBanks256 => 256,
      RomBanks512 => 512,
    }
  }
  pub fn as_usize(&self) -> usize {
    self.banks() * ROM_BANK_SIZE
  }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CartridgeRamSize {
  NoRam = 0x00,
  Ram2K = 0x01,
  Ram8K = 0x02,
  Ram32K = 0x03,
  Ram128K = 0x04,
  Ram64K = 0x05,
}

impl fmt::Debug for CartridgeRamSize {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    use self::CartridgeRamSize::*;
    write!(
      f,
      "{}",
      match *self {
        NoRam => "-",
        Ram2K => "16 kbit",
        Ram8K => "64 kbit",
        Ram32K => "256 kbit",
        Ram128K => "1 Mbit",
        Ram64K => "512 kbit",
      }
    )
  }
}

impl CartridgeRamSize {
  fn from_u8(val: u8) -> Result<CartridgeRamSize> {
    use self::CartridgeRamSize::*;
    match val {
      0x00 => Ok(NoRam),
      0x01 => Ok(Ram2K),
      0x02 => Ok(Ram8K),
      0x03 => Ok(Ram32K),
      0x04 => Ok(Ram128K),
      0x05 => Ok(Ram64K),
      _ => bail!("Invalid ram size {}.", val),
    }
  }
  pub fn as_usize(&self) -> usize {
    use self::CartridgeRamSize::*;
    match *self {
      NoRam => 0,
      Ram2K => 2048,
      Ram8K => 8192,
      Ram32K => 32768,
      Ram128K => 131_072,
      Ram64K => 65536,
    }
  }
}