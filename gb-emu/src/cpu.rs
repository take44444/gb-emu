use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
  cpu::{
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

macro_rules! step {
  ($s:expr, $d:expr, {$($c:tt : $e:expr,)*}) => {
    $(if $s == $c { $e })* else { return $d; }
  };
}
pub(crate) use step;
macro_rules! go {
  ($s:expr, $e:expr) => {
    $s = $e
  }
}
pub(crate) use go;

#[derive(Default, Clone, Serialize, Deserialize)]
struct Cache {
  step: u8,
  val8: u8,
  val16: u16,
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct Ctx {
  opcode: u8,
  cb: bool,
  int: bool,
  cache: HashMap<String, Cache>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Cpu {
  regs: Registers,
  pub interrupts: Interrupts,
  ctx: Ctx,
}

impl Cpu {
  pub fn new() -> Self {
    let mut ctx = Ctx::default();
    {
      ctx.cache.insert(String::from("call_isr"), Cache::default());
      ctx.cache.insert(String::from("inst"), Cache::default());
      ctx.cache.insert(String::from("push16"), Cache::default());
      ctx.cache.insert(String::from("pop16"), Cache::default());
      ctx.cache.insert(String::from("read8"), Cache::default());
      ctx.cache.insert(String::from("imm8"), Cache::default());
      ctx.cache.insert(String::from("read16"), Cache::default());
      ctx.cache.insert(String::from("write8"), Cache::default());
      ctx.cache.insert(String::from("write16"), Cache::default());
    }
    Self {
      regs: Registers::default(),
      interrupts: Interrupts::default(),
      ctx,
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
    step!(self.ctx.cache["call_isr"].step, (), {
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
        return go!(self.ctx.cache.get_mut("call_isr").unwrap().step, 1);
      },
      1: {
        self.interrupts.ime = false;
        go!(self.ctx.cache.get_mut("call_isr").unwrap().step, 0);
        self.fetch(bus)
      },
    });
  }
}
