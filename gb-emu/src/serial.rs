use serde::{Deserialize, Serialize};

use crate::cpu::interrupts::{Interrupts, self};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Serial {
  pub data: u8,
  control: u8,
  send_data: Option<u8>,
  recv_data: Option<u8>,
}

impl Serial {
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
  pub fn emulate_cycle(&mut self, interrupts: &mut Interrupts) {
    if self.recv_data.is_some() {
      self.data = self.recv_data.take().unwrap();
      self.control &= 0x7F;
      interrupts.irq(interrupts::SERIAL);
    }
  }
  pub fn send(&mut self) -> Option<u8> {
    self.send_data.take()
  }
  pub fn recv(&mut self, val: u8) {
    self.recv_data = Some(val);
  }
  pub fn is_master(&self) -> bool {
    self.control & 1 > 0
  }
}