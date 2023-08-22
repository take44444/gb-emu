use std::sync::Arc;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Model {
  // Dmg0,
  Dmg,
  // Mgb,
  // Sgb,
  // Sgb2,
}

pub struct Bootrom {
  model: Model,
  data: Arc<[u8]>,
  is_active: bool,
}

impl Bootrom {
  pub fn new(data: Arc<[u8]>) -> Self {
    if data.len() != 0x100 {
      panic!("Expected data size is 256, but it is ${:04x}", data.len());
    }
    Self {
      model: Model::Dmg,
      data: data,
      is_active: true,
    }
  }
  fn read(&self, addr: u16) -> u8 {
    self.data[addr as usize]
  }
}
