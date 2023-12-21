use serde::{Deserialize, Serialize};

use crate::cpu::interrupts::{self, Interrupts};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum Button {
  Down,
  Up,
  Left,
  Right,
  Start,
  Select,
  B,
  A,
}

impl Button {
  fn as_direction(&self) -> u8 {
    match self {
      Button::Down  => 0b1000,
      Button::Up    => 0b100,
      Button::Left  => 0b10,
      Button::Right => 0b1,
      _ => 0,
    }
  }
  fn as_action(&self) -> u8 {
    match self {
      Button::Start  => 0b1000,
      Button::Select => 0b100,
      Button::B      => 0b10,
      Button::A      => 0b1,
      _ => 0,
    }
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Joypad {
  mode: u8,
  action: u8,
  direction: u8,
}

impl Joypad {
  pub fn new() -> Self {
    Self {
      mode: 0,
      action: 0xFF,
      direction: 0xFF,
    }
  }
  pub fn read(&self) -> u8 {
    let mut ret = 0xCF | self.mode;
    if ret & 0x10 == 0 {
      ret &= self.direction;
    }
    if ret & 0x20 == 0 {
      ret &= self.action;
    }
    ret
  }
  pub fn write(&mut self, _: u16, val: u8) {
    self.mode = 0x30 & val;
  }
  pub fn button_down(&mut self, interrupts: &mut Interrupts, button: Button) {
    self.direction &= !button.as_direction();
    self.action &= !button.as_action();
    interrupts.irq(interrupts::JOYPAD);
  }
  pub fn button_up(&mut self, button: Button) {
    self.direction |= button.as_direction();
    self.action |= button.as_action();
  }
}
