use crate::{
  cpu::{
    Cpu,
    step,
    go,
    operand::{Reg16, Imm16, Imm8, Cond, IO8, IO16},
  },
  peripherals::Peripherals,
};

impl Cpu {
  pub fn push16(&mut self, bus: &mut Peripherals, val: u16) -> Option<()> {
    step!(self.ctx.cache["push16"].step, None, {
      0: {
        go!(self.ctx.cache.get_mut("push16").unwrap().step, 1);
        return None;
      },
      1: {
        let [lo, hi] = u16::to_le_bytes(val);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        bus.write(&mut self.interrupts, self.regs.sp, hi);
        self.ctx.cache.get_mut("push16").unwrap().val8 = lo;
        go!(self.ctx.cache.get_mut("push16").unwrap().step, 2);
        return None;
      },
      2: {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        bus.write(&mut self.interrupts, self.regs.sp, self.ctx.cache["push16"].val8);
        go!(self.ctx.cache.get_mut("push16").unwrap().step, 3);
        return None;
      },
      3: return Some(go!(self.ctx.cache.get_mut("push16").unwrap().step, 0)),
    });
  }
  pub fn pop16(&mut self, bus: &Peripherals) -> Option<u16> {
    step!(self.ctx.cache["pop16"].step, None, {
      0: {
        self.ctx.cache.get_mut("pop16").unwrap().val8 = bus.read(&self.interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        go!(self.ctx.cache.get_mut("pop16").unwrap().step, 1);
        return None;
      },
      1: {
        let hi = bus.read(&self.interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.ctx.cache.get_mut("pop16").unwrap().val16 = u16::from_le_bytes([self.ctx.cache["pop16"].val8, hi]);
        go!(self.ctx.cache.get_mut("pop16").unwrap().step, 2);
        return None;
      },
      2: {
        go!(self.ctx.cache.get_mut("pop16").unwrap().step, 0);
        return Some(self.ctx.cache.get_mut("pop16").unwrap().val16);
      },
    });
  }
  fn cond(&self, cond: Cond) -> bool {
    match cond {
      Cond::NZ => !self.regs.zf(),
      Cond::Z => self.regs.zf(),
      Cond::NC => !self.regs.cf(),
      Cond::C => self.regs.cf(),
    }
  }
  fn sub_general(&mut self, val: u8, carry: bool) -> u8 {
    let cy = carry as u8;
    let result = self.regs.a.wrapping_sub(val).wrapping_sub(cy);
    self.regs.set_zf(result == 0);
    self.regs.set_nf(true);
    self.regs.set_hf((self.regs.a & 0xf) < (val & 0xf) + cy);
    self.regs.set_cf((self.regs.a as u16) < (val as u16) + (cy as u16));
    result
  }
  fn rlc_general(&mut self, val: u8) -> u8 {
    self.regs.set_zf(val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(val & 0x80 > 0);
    (val << 1) | (val >> 7)
  }
  fn rl_general(&mut self, val: u8) -> u8 {
    let new_val = (val << 1) | self.regs.cf() as u8;
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(val & 0x80 > 0);
    new_val
  }
  fn rrc_general(&mut self, val: u8) -> u8 {
    self.regs.set_zf(val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(val & 1 > 0);
    (val << 7) | (val >> 1)
  }
  fn rr_general(&mut self, val: u8) -> u8 {
    let new_val = ((self.regs.cf() as u8) << 7) | (val >> 1);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(val & 1 > 0);
    new_val
  }
  // 8-bit operations
  pub fn ld<D: Copy, S: Copy>(&mut self, bus: &mut Peripherals, dst: D, src: S)
  where Self: IO8<D> + IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = v;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, dst, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 2);
      },
      2: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn add<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      let (result, carry) = self.regs.a.overflowing_add(v);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(false);
      self.regs.set_hf((self.regs.a & 0xf) + (v & 0xf) > 0xf);
      self.regs.set_cf(carry);
      self.regs.a = result;
      self.fetch(bus);
    }
  }
  pub fn adc<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    let cy = self.regs.cf() as u8;
    if let Some(v) = self.read8(bus, src) {
      let result = self.regs.a.wrapping_add(v).wrapping_add(cy);
      self.regs.set_zf(result == 0);
      self.regs.set_nf(false);
      self.regs.set_hf((self.regs.a & 0xf) + (v & 0xf) + cy > 0xf);
      self.regs.set_cf(self.regs.a as u16 + v as u16 + cy as u16 > 0xff);
      self.regs.a = result;
      self.fetch(bus);
    }
  }
  pub fn sub<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a = self.sub_general(v, false);
      self.fetch(bus);
    }
  }
  pub fn sbc<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a = self.sub_general(v, self.regs.cf());
      self.fetch(bus);
    }
  }
  pub fn cp<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      self.sub_general(v, false);
      self.fetch(bus);
    }
  }
  pub fn and<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a &= v;
      self.regs.set_zf(self.regs.a == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(true);
      self.regs.set_cf(false);
      self.fetch(bus);
    }
  }
  pub fn or<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a |= v;
      self.regs.set_zf(self.regs.a == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(false);
      self.regs.set_cf(false);
      self.fetch(bus);
    }
  }
  pub fn xor<S: Copy>(&mut self, bus: &Peripherals, src: S)
  where Self: IO8<S> {
    if let Some(v) = self.read8(bus, src) {
      self.regs.a ^= v;
      self.regs.set_zf(self.regs.a == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(false);
      self.regs.set_cf(false);
      self.fetch(bus);
    }
  }
  pub fn inc<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        let new_val = v.wrapping_add(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(v & 0xf == 0xf);
        self.ctx.cache.get_mut("inst").unwrap().val8 = new_val;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn dec<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        let new_val = v.wrapping_sub(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(v & 0xf == 0);
        self.ctx.cache.get_mut("inst").unwrap().val8 = new_val;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn rlca(&mut self, bus: &Peripherals) {
    self.regs.a = self.rlc_general(self.regs.a);
    self.regs.set_zf(false);
    self.fetch(bus);
  }
  pub fn rla(&mut self, bus: &Peripherals) {
    self.regs.a = self.rl_general(self.regs.a);
    self.regs.set_zf(false);
    self.fetch(bus);
  }
  pub fn rrca(&mut self, bus: &Peripherals) {
    self.regs.a = self.rrc_general(self.regs.a);
    self.regs.set_zf(false);
    self.fetch(bus);
  }
  pub fn rra(&mut self, bus: &Peripherals) {
    self.regs.a = self.rr_general(self.regs.a);
    self.regs.set_zf(false);
    self.fetch(bus);
  }
  pub fn rlc<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = self.rlc_general(v);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn rl<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = self.rl_general(v);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn rrc<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = self.rrc_general(v);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn rr<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = self.rr_general(v);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn sla<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v & 0x7f == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 0x80 > 0);
        self.ctx.cache.get_mut("inst").unwrap().val8 = v << 1;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn sra<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v & 0xFE == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 1 > 0);
        self.ctx.cache.get_mut("inst").unwrap().val8 = (v & 0x80) | (v >> 1);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn srl<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v & 0xFE == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(v & 1 > 0);
        self.ctx.cache.get_mut("inst").unwrap().val8 = v >> 1;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn swap<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.regs.set_zf(v == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.ctx.cache.get_mut("inst").unwrap().val8 = (v << 4) | (v >> 4);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn bit<S: Copy>(&mut self, bus: &Peripherals, bit: usize, src: S)
  where Self: IO8<S> {
    if let Some(mut v) = self.read8(bus, src) {
      v &= 1 << bit;
      self.regs.set_zf(v == 0);
      self.regs.set_nf(false);
      self.regs.set_hf(true);
      self.fetch(bus);
    }
  }
  pub fn set<S: Copy>(&mut self, bus: &mut Peripherals, bit: usize, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = v | (1 << bit);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn res<S: Copy>(&mut self, bus: &mut Peripherals, bit: usize, src: S)
  where Self: IO8<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val8 = v & !(1 << bit);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write8(bus, src, self.ctx.cache["inst"].val8).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn jp(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        self.regs.pc = v;
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn jp_hl(&mut self, bus: &Peripherals) {
    self.regs.pc = self.regs.hl();
    self.fetch(bus);
  }
  pub fn jr(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        self.regs.pc = self.regs.pc.wrapping_add(v as i8 as u16);
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn call(&mut self, bus: &mut Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        self.ctx.cache.get_mut("inst").unwrap().val16 = v;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.push16(bus, self.regs.pc).is_some() {
        self.regs.pc = self.ctx.cache.get_mut("inst").unwrap().val16;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn ret(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.pop16(bus) {
        self.regs.pc = v;
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn reti(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.pop16(bus) {
        self.regs.pc = v;
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        self.interrupts.ime = true;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn jp_c(&mut self, bus: &Peripherals, cond: Cond) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
        if self.cond(cond) {
          self.regs.pc = v;
          return;
        }
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn jr_c(&mut self, bus: &Peripherals, cond: Cond) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
        if self.cond(cond) {
          self.regs.pc = self.regs.pc.wrapping_add(v as i8 as u16);
          return;
        }
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn call_c(&mut self, bus: &mut Peripherals, cond: Cond) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, Imm16) {
        self.ctx.cache.get_mut("inst").unwrap().val16 = v;
        if self.cond(cond) {
          go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
        } else {
          self.fetch(bus);
        }
      },
      1: if self.push16(bus, self.regs.pc).is_some() {
        self.regs.pc = self.ctx.cache.get_mut("inst").unwrap().val16;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn ret_c(&mut self, bus: &Peripherals, cond: Cond) {
    step!(self.ctx.cache["inst"].step, (), {
      0: return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1),
      1: go!(self.ctx.cache.get_mut("inst").unwrap().step, if self.cond(cond) { 2 } else { 3 }),
      2: if let Some(v) = self.pop16(bus) {
        self.regs.pc = v;
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 3);
      },
      3: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn rst(&mut self, bus: &mut Peripherals, addr: u8) {
    if self.push16(bus, self.regs.pc).is_some() {
      self.regs.pc = addr as u16;
      self.fetch(bus);
    }
  }
  pub fn halt(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if self.interrupts.get_interrupt() > 0 {
        self.fetch(bus);
        // if self.interrupts.ime {
        //   self.fetch(bus);
        // } else {
        //   // This causes halt bug. (https://gbdev.io/pandocs/halt.html#halt-bug)
        //   self.ctx.opcode = bus.read(&self.interrupts, self.regs.pc);
        //   // self.fetch(bus);
        // }
      } else {
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        if self.interrupts.get_interrupt() > 0 {
          go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
          self.fetch(bus);
        }
      },
    });
  }
  pub fn stop(&mut self, _: &Peripherals) {
    panic!("STOP");
  }
  pub fn di(&mut self, bus: &Peripherals) {
    self.interrupts.ime = false;
    self.fetch(bus);
  }
  pub fn ei(&mut self, bus: &Peripherals) {
    self.fetch(bus);
    self.interrupts.ime = true;
  }
  pub fn ccf(&mut self, bus: &Peripherals) {
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(!self.regs.cf());
    self.fetch(bus);
  }
  pub fn scf(&mut self, bus: &Peripherals) {
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(true);
    self.fetch(bus);
  }
  pub fn nop(&mut self, bus: &Peripherals) {
    self.fetch(bus);
  }
  pub fn daa(&mut self, bus: &Peripherals) {
    let mut cf = false;
    if !self.regs.nf() {
      if self.regs.cf() || self.regs.a > 0x99 {
        cf = true;
        self.regs.a = self.regs.a.wrapping_add(0x60);
      }
      if self.regs.hf() || self.regs.a & 0x0f > 0x09 {
        self.regs.a = self.regs.a.wrapping_add(0x06);
      }
    } else {
      if self.regs.cf() {
        cf = true;
        if self.regs.hf() {
          self.regs.a = self.regs.a.wrapping_add(0x9A);
        } else {
          self.regs.a = self.regs.a.wrapping_add(0xA0);
        }
      } else if self.regs.hf() {
        self.regs.a = self.regs.a.wrapping_add(0xFA);
      }
    }
    self.regs.set_zf(self.regs.a == 0);
    self.regs.set_hf(false);
    self.regs.set_cf(cf);
    self.fetch(bus);
  }
  pub fn cpl(&mut self, bus: &Peripherals) {
    self.regs.a = !self.regs.a;
    self.regs.set_nf(true);
    self.regs.set_hf(true);
    self.fetch(bus);
  }
  // 16-bit operations
  pub fn ld16<D: Copy, S: Copy>(&mut self, bus: &mut Peripherals, dst: D, src: S)
  where Self: IO16<D> + IO16<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val16 = v;
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write16(bus, dst, self.ctx.cache["inst"].val16).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 2);
      },
      2: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn ld_sp_hl(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: {
        self.regs.sp = self.regs.hl();
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn ld_hl_sp_e(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        let val = v as i8 as u16;
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf((self.regs.sp & 0xF) + (val & 0xF) > 0xF);
        self.regs.set_cf((self.regs.sp & 0xFF) + (val & 0xFF) > 0xFF);
        self.regs.write_hl(self.regs.sp.wrapping_add(val));
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn push(&mut self, bus: &mut Peripherals, src: Reg16) {
    step!(self.ctx.cache["inst"].step, (), {
      0: {
        self.ctx.cache.get_mut("inst").unwrap().val16 = self.read16(bus, src).unwrap();
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.push16(bus, self.ctx.cache["inst"].val16).is_some() {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 2);
      },
      2: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn pop(&mut self, bus: &mut Peripherals, dst: Reg16) {
    if let Some(v) = self.pop16(bus) {
      self.write16(bus, dst, v);
      self.fetch(bus);
    }
  }
  pub fn add_hl_reg16(&mut self, bus: &Peripherals, src: Reg16) {
    step!(self.ctx.cache["inst"].step, (), {
      0: {
        let val = self.read16(bus, src).unwrap();
        let (result, carry) = self.regs.hl().overflowing_add(val);
        self.regs.set_nf(false);
        self.regs.set_hf((self.regs.hl() & 0xFFF) + (val & 0xFFF) > 0x0FFF);
        self.regs.set_cf(carry);
        self.regs.write_hl(result);
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn add_sp_e(&mut self, bus: &Peripherals) {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read8(bus, Imm8) {
        let val = v as i8 as u16;
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf((self.regs.sp & 0xF) + (val & 0xF) > 0xF);
        self.regs.set_cf((self.regs.sp & 0xFF) + (val & 0xFF) > 0xFF);
        self.regs.sp = self.regs.sp.wrapping_add(val);
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: return go!(self.ctx.cache.get_mut("inst").unwrap().step, 2),
      2: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn inc16<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO16<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val16 = v.wrapping_add(1);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write16(bus, src, self.ctx.cache["inst"].val16).is_some() {
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 2);
      },
      2: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn dec16<S: Copy>(&mut self, bus: &mut Peripherals, src: S)
  where Self: IO16<S> {
    step!(self.ctx.cache["inst"].step, (), {
      0: if let Some(v) = self.read16(bus, src) {
        self.ctx.cache.get_mut("inst").unwrap().val16 = v.wrapping_sub(1);
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 1);
      },
      1: if self.write16(bus, src, self.ctx.cache["inst"].val16).is_some() {
        return go!(self.ctx.cache.get_mut("inst").unwrap().step, 2);
      },
      2: {
        go!(self.ctx.cache.get_mut("inst").unwrap().step, 0);
        self.fetch(bus);
      },
    });
  }
  pub fn undefined(&mut self, _: &Peripherals) {
    panic!("Undefined opcode {:02x}", self.ctx.opcode);
  }
}
