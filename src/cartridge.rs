use std::fmt;
use anyhow::{Result, ensure};
use log::info;

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

impl CartridgeHeader {
  pub fn new(data: &Vec<u8>) -> Result<Self> {
    ensure!(data.len() >= 0x150, "Size of cartridge data must be more than 0x150.");
    let cartridge_header = unsafe {
      std::mem::transmute::<[u8; 0x50], CartridgeHeader>(
        data[0x100..0x150].try_into()?
      )
    };
    let mut chksum: u8 = 0;
    for i in 0x0134..0x014d {
      chksum = chksum.wrapping_sub(data[i]).wrapping_sub(1);
    }
    ensure!(chksum == cartridge_header.header_checksum[0], "Checksum validation failed.");
    info!("Checksum validation succeeded!");
    Ok(cartridge_header)
  }
}

pub struct Cartridge {

}
