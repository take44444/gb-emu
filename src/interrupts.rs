#[derive(Clone, Debug)]
pub struct Interrupts {
  pub intr_flags: u8,
  pub intr_enable: u8,
}

impl Interrupts {
  pub fn new() -> Interrupts {
    Interrupts {
      intr_flags: 0x00,
      intr_enable: 0x00,
    }
  }
  pub fn get_interrupt(&self) -> u8 {
    self.intr_flags & self.intr_enable
  }
  pub fn ack_interrupt(&mut self, mask: u8) {
    self.intr_flags = self.intr_flags & !mask;
  }
}