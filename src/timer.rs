use crate::cpu::interrupts;

pub struct Timer {
  div: u16,
  tima: u8,
  tma: u8,
  tac: u8,
  irq: Box<dyn Fn(u8)>,
  overflow: bool,
}

impl Timer {
  pub fn new(irq: Box<dyn Fn(u8)>) -> Self {
    Self {
      div: 0,
      tima: 0,
      tma: 0,
      tac: 0,
      irq,
      overflow: false,
    }
  }
  pub fn emulate_cycle(&mut self) {
    let modulo: u16 = match self.tac & 0b11 {
      0b01 => 1 << 3,
      0b10 => 1 << 5,
      0b11 => 1 << 7,
      _ => 1 << 9,
    };
    if self.overflow {
      self.div = self.div.wrapping_add(4);
      self.tima = self.tma;
      self.overflow = false;
      (self.irq)(interrupts::TIMER);
    } else if self.tac & 0b100 > 0 && self.div & modulo > 0 {
      self.div = self.div.wrapping_add(4);
      if self.div & modulo == 0 {
        let (tima, overflow) = self.tima.overflowing_add(1);
        self.tima = tima;
        self.overflow = overflow;
      }
    } else {
      self.div = self.div.wrapping_add(4);
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
      0xFF07 => self.tac = val & 0b00000111,
      _      => unreachable!(),
    }
  }
}