use anyhow::{bail, ensure, Result};
use crc::crc32;

pub struct Bootrom {
  data: Box<[u8]>,
  active: bool,
}

impl Bootrom {
  pub fn new(data: Box<[u8]>) -> Result<Self> {
    ensure!(data.len() == 0x100,
      "Expected data size is 256, but it is ${:04x}", data.len()
    );
    match crc32::checksum_ieee(&data) {
      0x59C8_598E => (),
      _ => bail!("Invalid bootrom. Only DMG is supported."),
    };
    Ok(Self {
      data,
      active: true,
    })
  }
  pub fn is_active(&self) -> bool {
    self.active
  }
  pub fn read(&self, addr: u16) -> u8 {
    self.data[addr as usize]
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    if addr == 0xFF50 {
      self.active &= val == 0;
    }
  }
}
