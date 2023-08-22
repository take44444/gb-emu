use crate::cartridge;

#[derive(Debug, Clone)]
struct Mbc1State {
  ramg: bool,
  bank1: u8,
  bank2: u8,
  mode: bool,
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