use serde::{Deserialize, Serialize};

use crate::cpu::interrupts::{Interrupts, self};

#[derive(Clone, Serialize, Deserialize)]
pub struct Serial {
  pub data: u8,
  control: u8,
  transfer_cnt: usize,
  send_data: Option<u8>,
  recv_data: Option<u8>,
  is_cgb: bool,
}

impl Serial {
  pub fn new(is_cgb: bool) -> Self {
    Self {
      data: 0,
      control: 0,
      transfer_cnt: 0,
      send_data: None,
      recv_data: None,
      is_cgb,
    }
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
        if self.control & 0x81 == 0x81 {
          if self.send_data.is_some() {
            panic!("Now sending!!");
          }
          if self.control & 0b10 > 0 && self.is_cgb {
            self.transfer_cnt = 4;
          } else {
            self.transfer_cnt = 128;
          }
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
    if self.transfer_cnt == 0 {
      if self.send_data.is_some() {
        self.control &= 0x7F;
        interrupts.irq(interrupts::SERIAL);
      }
    } else {
      self.transfer_cnt -= 1;
    }
  }
  pub fn send(&mut self) -> Option<u8> {
    if self.transfer_cnt == 0 && self.send_data.is_some() {
      self.send_data.take()
    } else {
      None
    }
  }
  pub fn recv(&mut self, val: u8) {
    if self.recv_data.is_some() {
      panic!("Now sending!!");
    }
    self.recv_data = Some(val);
  }
}