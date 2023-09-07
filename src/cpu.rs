use std::{
  rc::Rc, cell::RefCell,
  sync::atomic::{
    AtomicU8,
    AtomicU16,
    Ordering::Relaxed,
  },
};

use crate::{
  cpu::{
    instructions::step,
    register::Registers,
    interrupts::{Interrupts, VBLANK, STAT, TIMER, SERIAL, JOYPAD},
  },
  
  peripherals::Peripherals,
};

mod register;
mod operand;
mod fetch;
mod decode;
mod instructions;
pub mod interrupts;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Event {
  #[default]
  None,
  Int,
  Halt,
}

#[derive(Default)]
struct Ctx {
  opcode: u8,
  cb: bool,
  event: Event,
}

pub struct Cpu {
  regs: Registers,
  interrupts: Rc<RefCell<Interrupts>>,
  ime: bool,
  ctx: Ctx,
}

impl Cpu {
  pub fn new(interrupts: Rc<RefCell<Interrupts>>) -> Self {
    Self {
      regs: Registers::default(),
      interrupts,
      ime: false,
      ctx: Ctx::default(),
    }
  }
  pub fn emulate_cycle(&mut self, bus: &mut Peripherals) {
    match self.ctx.event {
      Event::Int => self.int(bus),
      Event::Halt => {
        if self.interrupts.borrow().get_interrupt() > 0 {
          self.fetch(bus);
        }
      }
      Event::None => self.decode(bus),
    }
  }
  fn int(&mut self, bus: &mut Peripherals) {
    step!((), {
      0: if let Some(_) = self.push16(bus, self.regs.pc) {
        self.ime = false;
        // get highest priority interrupt
        let interrupt: u8 = 1 << self.interrupts.borrow().get_interrupt().trailing_zeros();
        self.interrupts.borrow_mut().iak(interrupt);
        self.regs.pc = match interrupt {
          VBLANK => 0x0040,
          STAT => 0x0048,
          TIMER => 0x0050,
          SERIAL => 0x0058,
          JOYPAD => 0x0060,
          _ => panic!("Invalid interrupt: {:02x}", interrupt),
        };
        STEP.fetch_add(1, Relaxed);
        return;
      },
      1: {
        STEP.store(0, Relaxed);
        self.fetch(bus)
      },
    });
  }
}
