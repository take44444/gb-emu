
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
    let ret = self.0;
    self.0 = self.0.map(|x| x.wrapping_add(1)).filter(|&x| (x as u8) < 0xA0);
    ret
  }
  pub fn is_running(&self) -> bool {
    self.0.is_some()
  }
}
