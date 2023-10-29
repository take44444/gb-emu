use crate::cpu::interrupts::{self, Interrupts};

#[derive(Default, Clone)]
pub struct Timer {
  div: u16,
  tima: u8,
  tma: u8,
  tac: u8,
  overflow: bool,
}

impl Timer {
  pub fn emulate_cycle(&mut self, interrupts: &mut Interrupts) {
    self.div = self.div.wrapping_add(4);
    let modulo: u16 = match self.tac & 0b11 {
      0b01 => 16,
      0b10 => 64,
      0b11 => 256,
      _    => 1024,
    };
    if self.overflow {
      self.tima = self.tma;
      self.overflow = false;
      interrupts.irq(interrupts::TIMER);
    } else if self.tac & 0b100 > 0 && self.div & (modulo - 1) == 0 {
      let (tima, overflow) = self.tima.overflowing_add(1);
      self.tima = tima;
      self.overflow = overflow;
    }
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0xFF04 => (self.div >> 8) as u8,
      0xFF05 => self.tima,
      0xFF06 => self.tma,
      0xFF07 => 0b11111000 | self.tac,
      _      => unreachable!(),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0xFF04 => self.div = 0,
      0xFF05 => if !self.overflow {
        self.tima = val;
      },
      0xFF06 => self.tma = val,
      0xFF07 => self.tac = val & 0b111,
      _      => unreachable!(),
    }
  }
}