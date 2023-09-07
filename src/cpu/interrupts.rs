pub const VBLANK: u8 = 1 << 0;
pub const STAT: u8 = 1 << 1;
pub const TIMER: u8 = 1 << 2;
pub const SERIAL: u8 = 1 << 3;
pub const JOYPAD: u8 = 1 << 4;

#[derive(Clone, Debug, Default)]
pub struct Interrupts {
  intr_flags: u8,
  intr_enable: u8,
}

impl Interrupts {
  pub fn get_interrupt(&self) -> u8 {
    self.intr_flags & self.intr_enable & 0b00011111
  }
  pub fn irq(&mut self, val: u8) {
    self.intr_flags |= val & 0b00011111;
  }
  pub fn iak(&mut self, mask: u8) {
    self.intr_flags &= !mask;
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0xFF0F => self.intr_flags | 0b11100000,
      0xFFFF => self.intr_enable,
      _      => unreachable!(),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0xFF0F => self.intr_flags = val & 0b00011111,
      0xFFFF => self.intr_enable = val,
      _      => unreachable!(),
    }
  }
}