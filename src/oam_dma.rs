
#[derive(Clone)]
pub struct OamDma(Option<u16>);

impl OamDma {
  pub fn new() -> OamDma {
    OamDma(None)
  }
  pub fn request(&mut self, val: u8) {
    assert!(val <= 0xDF);
    self.0 =  Some((val as u16) << 8);
  }
  pub fn addr(&mut self) -> Option<u16> {
    if let Some(addr) = self.0 {
      let next_addr = addr.wrapping_add(1);
      if next_addr as u8 >= 0xA0 {
        self.0 = None;
      } else {
        self.0 = Some(next_addr);
      }
      Some(addr)
    } else {
      None
    }
  }
  pub fn is_running(&self) -> bool {
    self.0.is_some()
  }
}
