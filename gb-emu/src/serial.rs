use std::rc::Rc;

use crate::cpu::interrupts::{Interrupts, self};

#[derive(Clone)]
pub struct Serial {
  pub data: u8,
  control: u8,
  send_data: Option<u8>,
  recv_data: Option<u8>,
  send: Option<Rc<dyn Fn(u8)>>,
}

impl Serial {
  pub fn new() -> Self {
    Self {
      data: 0,
      control: 0,
      send_data: None,
      recv_data: None,
      send: None,
    }
  }
  pub fn set_callback(&mut self, send: Rc<dyn Fn(u8)>) {
    self.send = Some(send);
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
        if self.is_master() && self.control & 0x80 > 0 {
          self.send_data = Some(self.data);
        }
      }
      _      => unreachable!(),
    }
  }
  pub fn emulate_cycle(&mut self, interrupts: &mut Interrupts/*, rollback: usize*/) {
    if self.send_data.is_some() {
      if self.send.is_some() {
        (self.send.as_ref().unwrap())(self.send_data.unwrap()/*, rollback*/);
      } else {
        self.recv_data = Some(0xFF);
      }
      self.send_data = None;
    }
    if self.recv_data.is_some() {
      self.data = self.recv_data.take().unwrap();
      self.control &= 0x7F;
      interrupts.irq(interrupts::SERIAL);
    }
  }
  pub fn receive(&mut self, val: u8) {
    self.recv_data = Some(val);
  }
  pub fn is_master(&self) -> bool {
    self.control & 1 > 0
  }
}