use std::sync::atomic::{
  AtomicU8,
  AtomicU16,
  Ordering::Relaxed,
};

use crate::{
  cpu::{
    instructions::{step, go},
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

#[derive(Default, Clone)]
struct Ctx {
  opcode: u8,
  cb: bool,
  int: bool,
}

#[derive(Clone)]
pub struct Cpu {
  regs: Registers,
  pub interrupts: Interrupts,
  ctx: Ctx,
}

impl Cpu {
  pub fn new() -> Self {
    Self {
      regs: Registers::default(),
      interrupts: Interrupts::default(),
      ctx: Ctx::default(),
    }
  }
  pub fn emulate_cycle(&mut self, bus: &mut Peripherals) {
    if self.ctx.int {
      self.call_isr(bus);
    } else {
      self.decode(bus);
    }
  }
  fn call_isr(&mut self, bus: &mut Peripherals) {
    step!((), {
      0: if let Some(_) = self.push16(bus, self.regs.pc) {
        let highest_int: u8 = 1 << self.interrupts.get_interrupt().trailing_zeros();
        self.interrupts.intr_flags &= !highest_int;
        self.regs.pc = match highest_int {
          VBLANK => 0x0040,
          STAT   => 0x0048,
          TIMER  => 0x0050,
          SERIAL => 0x0058,
          JOYPAD => 0x0060,
          _ => panic!("Invalid interrupt: {:02x}", highest_int),
        };
        return go!(1);
      },
      1: {
        self.interrupts.ime = false;
        go!(0);
        self.fetch(bus)
      },
    });
  }
}
