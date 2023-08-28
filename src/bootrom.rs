use std::sync::Arc;
use anyhow::{bail, ensure, Result};
use crc::crc32;

pub struct Bootrom {
  data: Arc<[u8]>,
  pub is_active: bool,
}

impl Bootrom {
  pub fn new(data: Arc<[u8]>) -> Result<Self> {
    ensure!(data.len() == 0x100,
      "Expected data size is 256, but it is ${:04x}", data.len()
    );
    match crc32::checksum_ieee(&data) {
      0x59C8_598E => (),
      _ => bail!("Invalid bootrom. Only DMG is supported."),
    };
    Ok(Self {
      data,
      is_active: true,
    })
  }
  pub fn read(&self, addr: u16) -> u8 {
    self.data[addr as usize]
  }
}
