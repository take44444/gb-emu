pub const VBLANK: u8 = 1 << 0;
pub const STAT: u8 = 1 << 1;
pub const TIMER: u8 = 1 << 2;
pub const SERIAL: u8 = 1 << 3;
pub const JOYPAD: u8 = 1 << 4;

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