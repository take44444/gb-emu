use std::rc::Rc;

use crate::cpu::interrupts::{Interrupts, self};

#[derive(Clone)]
pub struct Serial {
  pub data: u8,
  control: u8,
  send: Option<Rc<dyn Fn(u8)>>,
}

impl Serial {
  pub fn new() -> Self {
    Self {
      data: 0,
      control: 0,
      send: None,
    }
  }
  pub fn set_callback(&mut self, callback: Rc<dyn Fn(u8)>) {
    self.send = Some(callback);
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0xFF01 => self.data,
      0xFF02 => self.control,
      _      => unreachable!(),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0xFF01 => self.data = val,
      0xFF02 => {
        self.control = val;
        if self.control & 1 > 0 && self.control & 0x80 > 0 {
          self.send.as_ref().map(|f| f(self.data));
        }
      }
      _      => unreachable!(),
    }
  }
  pub fn receive(&mut self, interrupts: &mut Interrupts, val: u8) {
    self.data = val;
    self.control &= 0x7F;
    interrupts.irq(interrupts::SERIAL);
  }
  pub fn is_master(&self) -> bool {
    self.control & 1 > 0
  }
}