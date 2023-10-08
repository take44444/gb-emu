use std::str;

use mbc::Mbc;

mod mbc;

#[repr(C)]
pub struct CartridgeHeader {
  entry_point: [u8; 4],
  logo: [u8; 48],
  title: [u8; 11],
  maker: [u8; 4],
  cgb_flag: [u8; 1],
  new_licensee: [u8; 2],
  sgb_flag: [u8; 1],
  cartridge_type: [u8; 1],
  rom_size: [u8; 1],
  sram_size: [u8; 1],
  destination: [u8; 1],
  old_licensee: [u8; 1],
  game_version: [u8; 1],
  header_checksum: [u8; 1],
  global_checksum: [u8; 2],
}

impl CartridgeHeader {
  fn new(data: [u8; 0x50]) -> Self {
    let ret = unsafe {
      std::mem::transmute::<[u8; 0x50], Self>(data)
    };
    let mut chksum: u8 = 0;
    for i in 0x34..0x4d {
      chksum = chksum.wrapping_sub(data[i]).wrapping_sub(1);
    }
    assert!(chksum == ret.header_checksum[0], "Checksum validation failed.");
    ret
  }
  fn rom_size(&self) -> usize {
    assert!(self.rom_size[0] <= 0x08, "Invalid rom size {}.", self.rom_size[0]);
    return 1 << (15 + self.rom_size[0]);
  }
  fn sram_size(&self) -> usize {
    match self.sram_size[0] {
      0x00 => 0,
      0x01 => 0x800,
      0x02 => 0x2000,
      0x03 => 0x8000,
      0x04 => 0x20000,
      0x05 => 0x10000,
      _    => panic!("Invalid sram size {}.", self.sram_size[0]),
    }
  }
}

pub struct Cartridge {
  rom: Box<[u8]>,
  sram: Box<[u8]>,
  mbc: Mbc,
}

impl Cartridge {
  pub fn new(data: Box<[u8]>) -> Self {
    let header = CartridgeHeader::new(data[0x100..0x150].try_into().unwrap());

    let title = str::from_utf8(&header.title).unwrap().trim_end_matches('\0').to_string();
    let rom_size = header.rom_size();
    let sram_size = header.sram_size();
    let rom_banks = rom_size >> 14; // ROMバンクは1つあたり16 KiB
    let mbc = Mbc::new(header.cartridge_type[0], rom_banks);

    println!("cartridge info {{ title: {}, type: {}, rom_size: {} B, sram_size: {} B }}",
      title,
      match mbc {
        Mbc::NoMbc { .. } => "NO MBC",
        Mbc::Mbc1 { .. } => "MBC1",
      },
      rom_size,
      sram_size,
    );
    assert!(data.len() == rom_size, "Expected {} bytes of cartridge ROM, got {}", rom_size, data.len());

    Self {
      rom: data,
      sram: vec![0; sram_size].into(),
      mbc,
    }
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0x0000..=0x7FFF => self.rom[self.mbc.get_addr(addr) & (self.rom.len() - 1)],
      0xA000..=0xBFFF => match self.mbc {
        Mbc::NoMbc => self.sram[addr as usize & (self.sram.len() - 1)],
        Mbc::Mbc1 { ref sram_enable, .. } => if *sram_enable {
          self.sram[self.mbc.get_addr(addr) & (self.sram.len() - 1)]
        } else {
          0xFF
        },
      },
      _               => unreachable!(),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0x0000..=0x7FFF => self.mbc.write(addr, val),
      0xA000..=0xBFFF => match self.mbc {
        Mbc::NoMbc => self.sram[addr as usize & (self.sram.len() - 1)] = val,
        Mbc::Mbc1 { ref sram_enable, .. } => if *sram_enable {
          self.sram[self.mbc.get_addr(addr) & (self.sram.len() - 1)] = val;
        },
      },
      _               => unreachable!(),
    }
  }
}
