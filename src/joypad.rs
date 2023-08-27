use crate::interrupts;

pub const P15: u8 = 1 << 5;
pub const P14: u8 = 1 << 4;
pub const P13: u8 = 1 << 3;
pub const P12: u8 = 1 << 2;
pub const P11: u8 = 1 << 1;
pub const P10: u8 = 1 << 0;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
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
  fn to_p1_direction(&self) -> u8 {
    match self {
      Button::Down => P13,
      Button::Up => P12,
      Button::Left => P11,
      Button::Right => P10,
      _ => 0,
    }
  }
  fn to_p1_action(&self) -> u8 {
    match self {
      Button::Start => P13,
      Button::Select => P12,
      Button::B => P11,
      Button::A => P10,
      _ => 0,
    }
  }
}

#[derive(Clone)]
pub struct Joypad {
  register: u8,
  action: u8,
  direction: u8,
}

impl Joypad {
  pub fn new() -> Self {
    Self {
      register: 0xCF,
      action: 0xFF,
      direction: 0xFF,
    }
  }
  pub fn read(&self) -> u8 {
    self.register
  }
  pub fn write(&mut self, val: u8) {
    self.register = (self.register & 0xCF) | ((P14 | P15) & val);
    self.action_direction();
  }
  pub fn button_down(&mut self, interrupts: &mut interrupts::Interrupts, button: Button) {
    self.direction &= !button.to_p1_direction();
    self.action &= !button.to_p1_action();
    self.action_direction();
    interrupts.req_interrupt(interrupts::JOYPAD);
  }
  pub fn button_up(&mut self, button: Button) {
    self.direction |= button.to_p1_direction();
    self.action |= button.to_p1_action();
    self.action_direction();
  }
  pub fn action_direction(&mut self) {
    self.register |= 0x0F;
    if self.register & P14 == 0 {
      self.register &= self.direction;
    }
    if self.register & P15 == 0 {
      self.register &= self.action;
    }
  }
}
