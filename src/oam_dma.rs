
#[derive(Clone)]
pub struct OamDma {
  requested_addr: Option<u16>,
  addr: Option<u16>,
}

impl OamDma {
  pub fn new() -> OamDma {
    OamDma {
      requested_addr: None,
      addr: None,
    }
  }
  pub fn request(&mut self, val: u8) {
    self.requested_addr =  Some((val as u16) << 8);
  }
  pub fn start_if_requested(&mut self) {
    if let Some(addr) = self.requested_addr.take() {
      self.addr = Some(addr);
    }
  }
  pub fn stop(&mut self) {
    self.addr = None;
  }
  pub fn addr(&mut self) -> Option<u16> {
    if let Some(addr) = self.addr {
      let next_addr = addr.wrapping_add(1);
      if next_addr as u8 >= 0xA0 {
        self.stop();
      } else {
        self.addr = Some(next_addr);
      }
      Some(addr)
    } else {
      None
    }
  }
  pub fn is_running(&self) -> bool {
    self.addr.is_some()
  }
}
