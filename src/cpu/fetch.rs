use crate::{
  cpu::{Cpu, Event},
  peripherals::Peripherals,
};

impl Cpu {
  pub fn fetch(&mut self, bus: &Peripherals) {
    self.ctx.opcode = bus.read(self.regs.pc);
    if self.ime && self.interrupts.borrow().get_interrupt() > 0 {
      self.ctx.event = Event::Int;
    } else {
      self.regs.pc = self.regs.pc.wrapping_add(1);
      self.ctx.event = Event::None;
    }
    self.ctx.cb = false;
  }
}
