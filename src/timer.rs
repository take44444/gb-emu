use crate::interrupts;

#[derive(Clone)]
pub struct Timer {
  div: u16,
  tima: u8,
  tma: u8,
  tac: u8,
  overflow: bool,
}

impl Timer {
  pub fn new() -> Self {
    Self {
      div: 0,
      tima: 0,
      tma: 0,
      tac: 0,
      overflow: false,
    }
  }
  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) {
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
      interrupts.req_interrupt(interrupts::TIMER);
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
  pub fn read_div(&self) -> u8 {
    (self.div >> 8) as u8
  }
  pub fn reset_div(&mut self) {
    self.div = 0;
  }
  pub fn read_tima(&self) -> u8 {
    self.tima
  }
  pub fn write_tima(&mut self, val: u8) {
    if !self.overflow {
      self.overflow = false;
      self.tima = val;
    }
  }
  pub fn read_tma(&self) -> u8 {
    self.tma
  }
  pub fn write_tma(&mut self, val: u8) {
    self.tma = val;
  }
  pub fn read_tac(&self) -> u8 {
    0b11111000 | self.tac
  }
  pub fn write_tac(&mut self, val: u8) {
    self.tac = val & 0b00000111;
  }
}