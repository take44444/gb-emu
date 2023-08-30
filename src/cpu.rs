use std::{cell::RefCell, rc::Rc};

use crate::{interrupts, peripherals, register, bus};

#[inline(always)]
fn check_add_carry(bit: usize, a: u16, b: u16) -> bool {
  let x = 1u16 << bit;
  let mask = x | x.wrapping_sub(1);
  (a & mask) + (b & mask) > mask
}

#[inline(always)]
fn isolate_rightmost_onebit(x: u8) -> u8 {
  let mask = (!x).wrapping_add(1); // -x
  x & mask
}

#[derive(Clone, Copy, Debug)]
enum Cond {
  NZ,
  Z,
  NC,
  C,
}

#[derive(Clone, Copy, Debug)]
enum Reg8 {
  A,
  B,
  C,
  D,
  E,
  H,
  L,
}

#[derive(Clone, Copy, Debug)]
enum Reg16 {
  AF,
  BC,
  DE,
  HL,
  SP,
}

#[derive(Clone, Copy, Debug)]
struct Imm8;

#[derive(Clone, Copy, Debug)]
struct Imm16;

#[derive(Clone, Copy, Debug)]
enum Indirect {
  BC,
  DE,
  HL,
  CFF,
  HLD,
  HLI,
}

#[derive(Clone, Copy, Debug)]
enum Direct8 {
  D,
  DFF,
}

#[derive(Clone, Copy, Debug)]
struct Direct16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
  Running,
  InterruptDispatch,
  Halt,
}

trait IO8<T: Copy> {
  fn read8(&mut self, interrupts: &interrupts::Interrupts, src: T) -> Option<u8>;
  fn write8(&mut self, interrupts: &mut interrupts::Interrupts, dst: T, val: u8);
}

trait IO16<T: Copy> {
  fn read16(&mut self, interrupts: &interrupts::Interrupts, src: T) -> Option<u16>;
  fn write16(&mut self, interrupts: &mut interrupts::Interrupts, dst: T, val: u16);
}

#[derive(Clone, Copy, Debug)]
enum Step {
  One(u8),
  Two(u8),
  Three(u8),
  Four(u8),
  Five(u8),
}

impl Step {
  fn cycle(&self) -> u8 {
    match *self {
      Self::One(x) => x,
      Self::Two(x) => x,
      Self::Three(x) => x,
      Self::Four(x) => x,
      Self::Five(x) => x,
    }
  }
  fn to_next(&mut self) {
    match self {
      Self::One(_) => *self = Self::Two(0),
      Self::Two(_) => *self = Self::Three(0),
      Self::Three(_) => *self = Self::Four(0),
      Self::Four(_) => *self = Self::Five(0),
      Self::Five(_) => unreachable!(),
    }
  }
  fn inc_cycle(&mut self) {
    match *self {
      Self::One(ref mut x) => *x += 1,
      Self::Two(ref mut x) => *x += 1,
      Self::Three(ref mut x) => *x += 1,
      Self::Four(ref mut x) => *x += 1,
      Self::Five(ref mut x) => *x += 1,
    }
  }
}

pub struct Cpu {
  regs: register::Registers,
  bus: bus::Bus,
  state: State,
  ime: bool,
  opcode: u8,
  instruction_step: Step,
  cb: bool,
  val8: u8,
  val16: u16,
  val8_io: u8,
  val16_io: u16,
}

impl IO8<Reg8> for Cpu {
  fn read8(&mut self, _: &interrupts::Interrupts, src: Reg8) -> Option<u8> {
    self.instruction_step.to_next();
    Some(match src {
      Reg8::A => self.regs.a,
      Reg8::B => self.regs.b,
      Reg8::C => self.regs.c,
      Reg8::D => self.regs.d,
      Reg8::E => self.regs.e,
      Reg8::H => self.regs.h,
      Reg8::L => self.regs.l,
    })
  }
  fn write8(&mut self, _: &mut interrupts::Interrupts, dst: Reg8, val: u8) {
    self.instruction_step.to_next();
    match dst {
      Reg8::A => self.regs.a = val,
      Reg8::B => self.regs.b = val,
      Reg8::C => self.regs.c = val,
      Reg8::D => self.regs.d = val,
      Reg8::E => self.regs.e = val,
      Reg8::H => self.regs.h = val,
      Reg8::L => self.regs.l = val,
    }
  }
}
impl IO16<Reg16> for Cpu {
  fn read16(&mut self, _: &interrupts::Interrupts, src: Reg16) -> Option<u16> {
    self.instruction_step.to_next();
    Some(self.read_r16(src))
  }
  fn write16(&mut self, _: &mut interrupts::Interrupts, dst: Reg16, val: u16) {
    self.instruction_step.to_next();
    self.write_r16(dst, val);
  }
}
impl IO8<Imm8> for Cpu {
  fn read8(&mut self, interrupts: &interrupts::Interrupts, _: Imm8) -> Option<u8> {
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = self.read_imm8(interrupts);
        self.instruction_step.inc_cycle();
        None
      },
      1 => {
        self.instruction_step.to_next();
        Some(self.val8_io)
      },
      _ => unreachable!(),
    }
  }
  fn write8(&mut self, _: &mut interrupts::Interrupts, _: Imm8, _: u8) {
    unreachable!()
  }
}
impl IO16<Imm16> for Cpu {
  fn read16(&mut self, interrupts: &interrupts::Interrupts, _: Imm16) -> Option<u16> {
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = self.read_imm8(interrupts);
        self.instruction_step.inc_cycle();
        None
      },
      1 => {
        let hi = self.read_imm8(interrupts);
        self.val16_io = u16::from_le_bytes([self.val8_io, hi]);
        self.instruction_step.inc_cycle();
        None
      },
      2 => {
        self.instruction_step.to_next();
        Some(self.val16_io)
      },
      _ => unreachable!(),
    }
  }
  fn write16(&mut self, _: &mut interrupts::Interrupts, _: Imm16, _: u16) {
    unreachable!()
  }
}
impl IO8<Indirect> for Cpu {
  fn read8(&mut self, interrupts: &interrupts::Interrupts, src: Indirect) -> Option<u8> {
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = match src {
          Indirect::BC => (self.bus.read)(interrupts, self.regs.bc()),
          Indirect::DE => (self.bus.read)(interrupts, self.regs.de()),
          Indirect::HL => (self.bus.read)(interrupts, self.regs.hl()),
          Indirect::CFF => (self.bus.read)(interrupts, 0xff00 | (self.regs.c as u16)),
          Indirect::HLD => {
            let addr = self.regs.hl();
            self.write_r16(Reg16::HL, addr.wrapping_sub(1));
            (self.bus.read)(interrupts, addr)
          },
          Indirect::HLI => {
            let addr = self.regs.hl();
            self.write_r16(Reg16::HL, addr.wrapping_add(1));
            (self.bus.read)(interrupts, addr)
          },
        };
        self.instruction_step.inc_cycle();
        None
      },
      1 => {
        self.instruction_step.to_next();
        Some(self.val8_io)
      },
      _ => unreachable!(),
    }
  }
  fn write8(&mut self, interrupts: &mut interrupts::Interrupts, dst: Indirect, val: u8) {
    match self.instruction_step.cycle() {
      0 => {
        match dst {
          Indirect::BC => (self.bus.write)(interrupts, self.regs.bc(), val),
          Indirect::DE => (self.bus.write)(interrupts, self.regs.de(), val),
          Indirect::HL => (self.bus.write)(interrupts, self.regs.hl(), val),
          Indirect::CFF => (self.bus.write)(interrupts, 0xff00 | (self.regs.c as u16), val),
          Indirect::HLD => {
            let addr = self.regs.hl();
            (self.bus.write)(interrupts, addr, val);
            self.write_r16(Reg16::HL, addr.wrapping_sub(1));
          },
          Indirect::HLI => {
            let addr = self.regs.hl();
            (self.bus.write)(interrupts, addr, val);
            self.write_r16(Reg16::HL, addr.wrapping_add(1));
          },
        }
        self.instruction_step.inc_cycle();
      },
      1 => self.instruction_step.to_next(),
      _ => unreachable!(),
    }
  }
}
impl IO8<Direct8> for Cpu {
  fn read8(&mut self, interrupts: &interrupts::Interrupts, src: Direct8) -> Option<u8> {
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = self.read_imm8(interrupts);
        if let Direct8::DFF = src {
          self.val16_io = 0xff00 | (self.val8_io as u16);
          self.instruction_step.inc_cycle();
        }
        self.instruction_step.inc_cycle();
        None
      },
      1 => {
        let hi = self.read_imm8(interrupts);
        self.val16_io = u16::from_le_bytes([self.val8_io, hi]);
        self.instruction_step.inc_cycle();
        None
      },
      2 => {
        self.val8_io = (self.bus.read)(interrupts, self.val16_io);
        self.instruction_step.inc_cycle();
        None
      },
      3 => {
        self.instruction_step.to_next();
        Some(self.val8_io)
      },
      _ => unreachable!(),
    }
  }
  fn write8(&mut self, interrupts: &mut interrupts::Interrupts, dst: Direct8, val: u8) {
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = self.read_imm8(interrupts);
        if let Direct8::DFF = dst {
          self.val16_io = 0xff00 | (self.val8_io as u16);
          self.instruction_step.inc_cycle();
        }
        self.instruction_step.inc_cycle();
      },
      1 => {
        let hi = self.read_imm8(interrupts);
        self.val16_io = u16::from_le_bytes([self.val8_io, hi]);
        self.instruction_step.inc_cycle();
      },
      2 => {
        (self.bus.write)(interrupts, self.val16_io, val);
        self.instruction_step.inc_cycle();
      },
      3 => self.instruction_step.to_next(),
      _ => unreachable!(),
    }
  }
}
impl IO16<Direct16> for Cpu {
  fn read16(&mut self, _: &interrupts::Interrupts, _: Direct16) -> Option<u16> {
    unreachable!()
  }
  fn write16(&mut self, interrupts: &mut interrupts::Interrupts, _: Direct16, val: u16) {
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = self.read_imm8(interrupts);
        self.instruction_step.inc_cycle();
      },
      1 => {
        let hi = self.read_imm8(interrupts);
        self.val16_io = u16::from_le_bytes([self.val8_io, hi]);
        self.instruction_step.inc_cycle();
      },
      2 => {
        (self.bus.write)(interrupts, self.val16_io, val as u8);
        self.instruction_step.inc_cycle();
      },
      3 => {
        (self.bus.write)(interrupts, self.val16_io.wrapping_add(1), (val >> 8) as u8);
        self.instruction_step.inc_cycle();
      },
      4 => self.instruction_step.to_next(),
      _ => unreachable!(),
    }
  }
}

macro_rules! step {
  ($e:expr, ) => {{}};
  ($e:expr, $m1:ident:$s1:stmt, $($rest:tt)*) => {
    if let Step::$m1(_) = $e { $s1 }
    else { step!($e, $($rest)*); }
  };
  ($e:expr, $m1:ident>$s1:stmt, $($rest:tt)*) => {
    if let Step::$m1(_) = $e { $s1 }
    step!($e, $($rest)*)
  };
}

impl Cpu {
  pub fn new(peripherals: Rc<RefCell<peripherals::Peripherals>>) -> Self {
    Self {
      regs: register::Registers::new(),
      bus: bus::Bus::new(peripherals),
      state: State::Running,
      ime: false,
      opcode: 0x00,
      instruction_step: Step::One(0),
      cb: false,
      val8: 0,
      val16: 0,
      val8_io: 0,
      val16_io: 0,
    }
  }

  fn prefetch(&mut self, interrupts: &interrupts::Interrupts) {
    self.opcode = (self.bus.read)(interrupts, self.regs.pc);
    let interrupt = interrupts.get_interrupt();
    if self.ime && interrupt != 0 {
      self.state = State::InterruptDispatch;
    } else {
      self.regs.pc = self.regs.pc.wrapping_add(1);
      self.state = State::Running;
    }
    self.instruction_step = Step::One(0);
    self.cb = false;
  }

  fn read_r16(&self, src: Reg16) -> u16 {
    match src {
      Reg16::AF => self.regs.af(),
      Reg16::BC => self.regs.bc(),
      Reg16::DE => self.regs.de(),
      Reg16::HL => self.regs.hl(),
      Reg16::SP => self.regs.sp,
    }
  }
  fn write_r16(&mut self, dst: Reg16, val: u16) {
    match dst {
      Reg16::AF => {
        self.regs.a = (val >> 8) as u8;
        self.regs.f = (val & 0xF0) as u8;
      },
      Reg16::BC => {
        self.regs.b = (val >> 8) as u8;
        self.regs.c = val as u8;
      },
      Reg16::DE => {
        self.regs.d = (val >> 8) as u8;
        self.regs.e = val as u8;
      },
      Reg16::HL => {
        self.regs.h = (val >> 8) as u8;
        self.regs.l = val as u8;
      },
      Reg16::SP => self.regs.sp = val,
    }
  }
  fn read_imm8(&mut self, interrupts: &interrupts::Interrupts) -> u8 {
    let ret = (self.bus.read)(interrupts, self.regs.pc);
    self.regs.pc = self.regs.pc.wrapping_add(1);
    ret
  }
  fn push16(&mut self, interrupts: &mut interrupts::Interrupts, val: u16) {
    match self.instruction_step.cycle() {
      0 => self.instruction_step.inc_cycle(),
      1 => {
        let [lo, hi] = u16::to_le_bytes(val);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        (self.bus.write)(interrupts, self.regs.sp, hi);
        self.val8_io = lo;
        self.instruction_step.inc_cycle();
      },
      2 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        (self.bus.write)(interrupts, self.regs.sp, self.val8_io);
        self.instruction_step.inc_cycle();
      },
      3 => self.instruction_step.to_next(),
      _ => unreachable!(),
    }
  }
  fn pop16(&mut self, interrupts: &mut interrupts::Interrupts) -> Option<u16>{
    match self.instruction_step.cycle() {
      0 => {
        self.val8_io = (self.bus.read)(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.instruction_step.inc_cycle();
        None
      },
      1 => {
        let hi = (self.bus.read)(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.val16_io = u16::from_le_bytes([self.val8_io, hi]);
        self.instruction_step.inc_cycle();
        None
      },
      2 => {
        self.instruction_step.to_next();
        Some(self.val16_io)
      },
      _ => unreachable!(),
    }
  }

  fn check_cond(&self, cond: Cond) -> bool {
    match cond {
      Cond::NZ => !self.regs.zf(),
      Cond::Z => self.regs.zf(),
      Cond::NC => !self.regs.cf(),
      Cond::C => self.regs.cf(),
    }
  }

  fn alu_sub(&mut self, val: u8, carry: bool) -> u8 {
    let cy = carry as u8;
    let result = self.regs.a.wrapping_sub(val).wrapping_sub(cy);
    self.regs.set_zf(result == 0);
    self.regs.set_nf(true);
    self.regs.set_hf(
      (self.regs.a & 0xf)
        .wrapping_sub(val & 0xf)
        .wrapping_sub(cy)
        & (0xf + 1)
        != 0,
    );
    self.regs.set_cf(
      (self.regs.a as u16) < (val as u16) + (cy as u16)
    );
    result
  }
  fn alu_rl(&mut self, val: u8) -> u8 {
    let ci = self.regs.cf() as u8;
    let co = val & 0x80;
    let new_val = (val << 1) | ci;
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co > 0);
    new_val
  }
  fn alu_rlc(&mut self, val: u8) -> u8 {
    let co = val & 0x80;
    let new_val = val.rotate_left(1);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co > 0);
    new_val
  }
  fn alu_rr(&mut self, val: u8) -> u8 {
    let ci = self.regs.cf() as u8;
    let co = val & 0x01;
    let new_val = (val >> 1) | (ci << 7);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co > 0);
    new_val
  }
  fn alu_rrc(&mut self, val: u8) -> u8 {
    let co = val & 0x01;
    let new_val = val.rotate_right(1);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co > 0);
    new_val
  }

  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) {
    match self.state {
      State::Running => self.decode_exec_fetch_cycle(interrupts),
      State::InterruptDispatch => self.interrupt_dispatch(interrupts),
      State::Halt => {
        if interrupts.get_interrupt() != 0 {
          self.prefetch(interrupts);
        }
      }
    }
  }
  // https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
  fn decode_exec_fetch_cycle(&mut self, interrupts: &mut interrupts::Interrupts) {
    if self.cb {
      self.cb_decode_exec_fetch_cycle(interrupts);
      return;
    }
    match self.opcode {
      0x00 => self.nop(interrupts),
      0x10 => self.stop(),
      0x20 => self.jr_cc(interrupts, Cond::NZ),
      0x30 => self.jr_cc(interrupts, Cond::NC),
      0x01 => self.ld16(interrupts, Reg16::BC, Imm16),
      0x11 => self.ld16(interrupts, Reg16::DE, Imm16),
      0x21 => self.ld16(interrupts, Reg16::HL, Imm16),
      0x31 => self.ld16(interrupts, Reg16::SP, Imm16),
      0x02 => self.ld(interrupts, Indirect::BC, Reg8::A),
      0x12 => self.ld(interrupts, Indirect::DE, Reg8::A),
      0x22 => self.ld(interrupts, Indirect::HLI, Reg8::A),
      0x32 => self.ld(interrupts, Indirect::HLD, Reg8::A),
      0x03 => self.inc16(interrupts, Reg16::BC),
      0x13 => self.inc16(interrupts, Reg16::DE),
      0x23 => self.inc16(interrupts, Reg16::HL),
      0x33 => self.inc16(interrupts, Reg16::SP),
      0x04 => self.inc(interrupts, Reg8::B),
      0x14 => self.inc(interrupts, Reg8::D),
      0x24 => self.inc(interrupts, Reg8::H),
      0x34 => self.inc(interrupts, Indirect::HL),
      0x05 => self.dec(interrupts, Reg8::B),
      0x15 => self.dec(interrupts, Reg8::D),
      0x25 => self.dec(interrupts, Reg8::H),
      0x35 => self.dec(interrupts, Indirect::HL),
      0x06 => self.ld(interrupts, Reg8::B, Imm8),
      0x16 => self.ld(interrupts, Reg8::D, Imm8),
      0x26 => self.ld(interrupts, Reg8::H, Imm8),
      0x36 => self.ld(interrupts, Indirect::HL, Imm8),
      0x07 => self.rlca(interrupts),
      0x17 => self.rla(interrupts),
      0x27 => self.daa(interrupts),
      0x37 => self.scf(interrupts),
      0x08 => self.ld16(interrupts, Direct16, Reg16::SP),
      0x18 => self.jr(interrupts),
      0x28 => self.jr_cc(interrupts, Cond::Z),
      0x38 => self.jr_cc(interrupts, Cond::C),
      0x09 => self.add_hl_r16(interrupts, Reg16::BC),
      0x19 => self.add_hl_r16(interrupts, Reg16::DE),
      0x29 => self.add_hl_r16(interrupts, Reg16::HL),
      0x39 => self.add_hl_r16(interrupts, Reg16::SP),
      0x0A => self.ld(interrupts, Reg8::A, Indirect::BC),
      0x1A => self.ld(interrupts, Reg8::A, Indirect::DE),
      0x2A => self.ld(interrupts, Reg8::A, Indirect::HLI),
      0x3A => self.ld(interrupts, Reg8::A, Indirect::HLD),
      0x0B => self.dec16(interrupts, Reg16::BC),
      0x1B => self.dec16(interrupts, Reg16::DE),
      0x2B => self.dec16(interrupts, Reg16::HL),
      0x3B => self.dec16(interrupts, Reg16::SP),
      0x0C => self.inc(interrupts, Reg8::C),
      0x1C => self.inc(interrupts, Reg8::E),
      0x2C => self.inc(interrupts, Reg8::L),
      0x3C => self.inc(interrupts, Reg8::A),
      0x0D => self.dec(interrupts, Reg8::C),
      0x1D => self.dec(interrupts, Reg8::E),
      0x2D => self.dec(interrupts, Reg8::L),
      0x3D => self.dec(interrupts, Reg8::A),
      0x0E => self.ld(interrupts, Reg8::C, Imm8),
      0x1E => self.ld(interrupts, Reg8::E, Imm8),
      0x2E => self.ld(interrupts, Reg8::L, Imm8),
      0x3E => self.ld(interrupts, Reg8::A, Imm8),
      0x0F => self.rrca(interrupts),
      0x1F => self.rra(interrupts),
      0x2F => self.cpl(interrupts),
      0x3F => self.ccf(interrupts),
      0x40 => self.ld(interrupts, Reg8::B, Reg8::B),
      0x50 => self.ld(interrupts, Reg8::D, Reg8::B),
      0x60 => self.ld(interrupts, Reg8::H, Reg8::B),
      0x70 => self.ld(interrupts, Indirect::HL, Reg8::B),
      0x41 => self.ld(interrupts, Reg8::B, Reg8::C),
      0x51 => self.ld(interrupts, Reg8::D, Reg8::C),
      0x61 => self.ld(interrupts, Reg8::H, Reg8::C),
      0x71 => self.ld(interrupts, Indirect::HL, Reg8::C),
      0x42 => self.ld(interrupts, Reg8::B, Reg8::D),
      0x52 => self.ld(interrupts, Reg8::D, Reg8::D),
      0x62 => self.ld(interrupts, Reg8::H, Reg8::D),
      0x72 => self.ld(interrupts, Indirect::HL, Reg8::D),
      0x43 => self.ld(interrupts, Reg8::B, Reg8::E),
      0x53 => self.ld(interrupts, Reg8::D, Reg8::E),
      0x63 => self.ld(interrupts, Reg8::H, Reg8::E),
      0x73 => self.ld(interrupts, Indirect::HL, Reg8::E),
      0x44 => self.ld(interrupts, Reg8::B, Reg8::H),
      0x54 => self.ld(interrupts, Reg8::D, Reg8::H),
      0x64 => self.ld(interrupts, Reg8::H, Reg8::H),
      0x74 => self.ld(interrupts, Indirect::HL, Reg8::H),
      0x45 => self.ld(interrupts, Reg8::B, Reg8::L),
      0x55 => self.ld(interrupts, Reg8::D, Reg8::L),
      0x65 => self.ld(interrupts, Reg8::H, Reg8::L),
      0x75 => self.ld(interrupts, Indirect::HL, Reg8::L),
      0x46 => self.ld(interrupts, Reg8::B, Indirect::HL),
      0x56 => self.ld(interrupts, Reg8::D, Indirect::HL),
      0x66 => self.ld(interrupts, Reg8::H, Indirect::HL),
      0x76 => self.halt(interrupts),
      0x47 => self.ld(interrupts, Reg8::B, Reg8::A),
      0x57 => self.ld(interrupts, Reg8::D, Reg8::A),
      0x67 => self.ld(interrupts, Reg8::H, Reg8::A),
      0x77 => self.ld(interrupts, Indirect::HL, Reg8::A),
      0x48 => self.ld(interrupts, Reg8::C, Reg8::B),
      0x58 => self.ld(interrupts, Reg8::E, Reg8::B),
      0x68 => self.ld(interrupts, Reg8::L, Reg8::B),
      0x78 => self.ld(interrupts, Reg8::A, Reg8::B),
      0x49 => self.ld(interrupts, Reg8::C, Reg8::C),
      0x59 => self.ld(interrupts, Reg8::E, Reg8::C),
      0x69 => self.ld(interrupts, Reg8::L, Reg8::C),
      0x79 => self.ld(interrupts, Reg8::A, Reg8::C),
      0x4A => self.ld(interrupts, Reg8::C, Reg8::D),
      0x5A => self.ld(interrupts, Reg8::E, Reg8::D),
      0x6A => self.ld(interrupts, Reg8::L, Reg8::D),
      0x7A => self.ld(interrupts, Reg8::A, Reg8::D),
      0x4B => self.ld(interrupts, Reg8::C, Reg8::E),
      0x5B => self.ld(interrupts, Reg8::E, Reg8::E),
      0x6B => self.ld(interrupts, Reg8::L, Reg8::E),
      0x7B => self.ld(interrupts, Reg8::A, Reg8::E),
      0x4C => self.ld(interrupts, Reg8::C, Reg8::H),
      0x5C => self.ld(interrupts, Reg8::E, Reg8::H),
      0x6C => self.ld(interrupts, Reg8::L, Reg8::H),
      0x7C => self.ld(interrupts, Reg8::A, Reg8::H),
      0x4D => self.ld(interrupts, Reg8::C, Reg8::L),
      0x5D => self.ld(interrupts, Reg8::E, Reg8::L),
      0x6D => self.ld(interrupts, Reg8::L, Reg8::L),
      0x7D => self.ld(interrupts, Reg8::A, Reg8::L),
      0x4E => self.ld(interrupts, Reg8::C, Indirect::HL),
      0x5E => self.ld(interrupts, Reg8::E, Indirect::HL),
      0x6E => self.ld(interrupts, Reg8::L, Indirect::HL),
      0x7E => self.ld(interrupts, Reg8::A, Indirect::HL),
      0x4F => self.ld(interrupts, Reg8::C, Reg8::A),
      0x5F => self.ld(interrupts, Reg8::E, Reg8::A),
      0x6F => self.ld(interrupts, Reg8::L, Reg8::A),
      0x7F => self.ld(interrupts, Reg8::A, Reg8::A),
      0x80 => self.add(interrupts, Reg8::B),
      0x90 => self.sub(interrupts, Reg8::B),
      0xA0 => self.and(interrupts, Reg8::B),
      0xB0 => self.or(interrupts, Reg8::B),
      0x81 => self.add(interrupts, Reg8::C),
      0x91 => self.sub(interrupts, Reg8::C),
      0xA1 => self.and(interrupts, Reg8::C),
      0xB1 => self.or(interrupts, Reg8::C),
      0x82 => self.add(interrupts, Reg8::D),
      0x92 => self.sub(interrupts, Reg8::D),
      0xA2 => self.and(interrupts, Reg8::D),
      0xB2 => self.or(interrupts, Reg8::D),
      0x83 => self.add(interrupts, Reg8::E),
      0x93 => self.sub(interrupts, Reg8::E),
      0xA3 => self.and(interrupts, Reg8::E),
      0xB3 => self.or(interrupts, Reg8::E),
      0x84 => self.add(interrupts, Reg8::H),
      0x94 => self.sub(interrupts, Reg8::H),
      0xA4 => self.and(interrupts, Reg8::H),
      0xB4 => self.or(interrupts, Reg8::H),
      0x85 => self.add(interrupts, Reg8::L),
      0x95 => self.sub(interrupts, Reg8::L),
      0xA5 => self.and(interrupts, Reg8::L),
      0xB5 => self.or(interrupts, Reg8::L),
      0x86 => self.add(interrupts, Indirect::HL),
      0x96 => self.sub(interrupts, Indirect::HL),
      0xA6 => self.and(interrupts, Indirect::HL),
      0xB6 => self.or(interrupts, Indirect::HL),
      0x87 => self.add(interrupts, Reg8::A),
      0x97 => self.sub(interrupts, Reg8::A),
      0xA7 => self.and(interrupts, Reg8::A),
      0xB7 => self.or(interrupts, Reg8::A),
      0x88 => self.adc(interrupts, Reg8::B),
      0x98 => self.sbc(interrupts, Reg8::B),
      0xA8 => self.xor(interrupts, Reg8::B),
      0xB8 => self.cp(interrupts, Reg8::B),
      0x89 => self.adc(interrupts, Reg8::C),
      0x99 => self.sbc(interrupts, Reg8::C),
      0xA9 => self.xor(interrupts, Reg8::C),
      0xB9 => self.cp(interrupts, Reg8::C),
      0x8A => self.adc(interrupts, Reg8::D),
      0x9A => self.sbc(interrupts, Reg8::D),
      0xAA => self.xor(interrupts, Reg8::D),
      0xBA => self.cp(interrupts, Reg8::D),
      0x8B => self.adc(interrupts, Reg8::E),
      0x9B => self.sbc(interrupts, Reg8::E),
      0xAB => self.xor(interrupts, Reg8::E),
      0xBB => self.cp(interrupts, Reg8::E),
      0x8C => self.adc(interrupts, Reg8::H),
      0x9C => self.sbc(interrupts, Reg8::H),
      0xAC => self.xor(interrupts, Reg8::H),
      0xBC => self.cp(interrupts, Reg8::H),
      0x8D => self.adc(interrupts, Reg8::L),
      0x9D => self.sbc(interrupts, Reg8::L),
      0xAD => self.xor(interrupts, Reg8::L),
      0xBD => self.cp(interrupts, Reg8::L),
      0x8E => self.adc(interrupts, Indirect::HL),
      0x9E => self.sbc(interrupts, Indirect::HL),
      0xAE => self.xor(interrupts, Indirect::HL),
      0xBE => self.cp(interrupts, Indirect::HL),
      0x8F => self.adc(interrupts, Reg8::A),
      0x9F => self.sbc(interrupts, Reg8::A),
      0xAF => self.xor(interrupts, Reg8::A),
      0xBF => self.cp(interrupts, Reg8::A),
      0xC0 => self.ret_cc(interrupts, Cond::NZ),
      0xD0 => self.ret_cc(interrupts, Cond::NC),
      0xE0 => self.ld(interrupts, Direct8::DFF, Reg8::A),
      0xF0 => self.ld(interrupts, Reg8::A, Direct8::DFF),
      0xC1 => self.pop_r16(interrupts, Reg16::BC),
      0xD1 => self.pop_r16(interrupts, Reg16::DE),
      0xE1 => self.pop_r16(interrupts, Reg16::HL),
      0xF1 => self.pop_r16(interrupts, Reg16::AF),
      0xC2 => self.jp_cc(interrupts, Cond::NZ),
      0xD2 => self.jp_cc(interrupts, Cond::NC),
      0xE2 => self.ld(interrupts, Indirect::CFF, Reg8::A),
      0xF2 => self.ld(interrupts, Reg8::A, Indirect::CFF),
      0xC3 => self.jp(interrupts),
      0xD3 => self.undefined(),
      0xE3 => self.undefined(),
      0xF3 => self.di(interrupts),
      0xC4 => self.call_cc(interrupts, Cond::NZ),
      0xD4 => self.call_cc(interrupts, Cond::NC),
      0xE4 => self.undefined(),
      0xF4 => self.undefined(),
      0xC5 => self.push_r16(interrupts, Reg16::BC),
      0xD5 => self.push_r16(interrupts, Reg16::DE),
      0xE5 => self.push_r16(interrupts, Reg16::HL),
      0xF5 => self.push_r16(interrupts, Reg16::AF),
      0xC6 => self.add(interrupts, Imm8),
      0xD6 => self.sub(interrupts, Imm8),
      0xE6 => self.and(interrupts, Imm8),
      0xF6 => self.or(interrupts, Imm8),
      0xC7 => self.rst(interrupts, 0x00),
      0xD7 => self.rst(interrupts, 0x10),
      0xE7 => self.rst(interrupts, 0x20),
      0xF7 => self.rst(interrupts, 0x30),
      0xC8 => self.ret_cc(interrupts, Cond::Z),
      0xD8 => self.ret_cc(interrupts, Cond::C),
      0xE8 => self.add_sp_e(interrupts),
      0xF8 => self.ld_hl_sp_e(interrupts),
      0xC9 => self.ret(interrupts),
      0xD9 => self.reti(interrupts),
      0xE9 => self.jp_hl(interrupts),
      0xF9 => self.ld_sp_hl(interrupts),
      0xCA => self.jp_cc(interrupts, Cond::Z),
      0xDA => self.jp_cc(interrupts, Cond::C),
      0xEA => self.ld(interrupts, Direct8::D, Reg8::A),
      0xFA => self.ld(interrupts, Reg8::A, Direct8::D),
      0xCB => self.cb_prefix(interrupts),
      0xDB => self.undefined(),
      0xEB => self.undefined(),
      0xFB => self.ei(interrupts),
      0xCC => self.call_cc(interrupts, Cond::Z),
      0xDC => self.call_cc(interrupts, Cond::C),
      0xEC => self.undefined(),
      0xFC => self.undefined(),
      0xCD => self.call(interrupts),
      0xDD => self.undefined(),
      0xED => self.undefined(),
      0xFD => self.undefined(),
      0xCE => self.adc(interrupts, Imm8),
      0xDE => self.sbc(interrupts, Imm8),
      0xEE => self.xor(interrupts, Imm8),
      0xFE => self.cp(interrupts, Imm8),
      0xCF => self.rst(interrupts, 0x08),
      0xDF => self.rst(interrupts, 0x18),
      0xEF => self.rst(interrupts, 0x28),
      0xFF => self.rst(interrupts, 0x38),
    }
  }
  // https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
  fn cb_decode_exec_fetch_cycle(&mut self, interrupts: &mut interrupts::Interrupts) {
    match self.opcode {
      0x00 => self.rlc(interrupts, Reg8::B),
      0x10 => self.rl(interrupts, Reg8::B),
      0x20 => self.sla(interrupts, Reg8::B),
      0x30 => self.swap(interrupts, Reg8::B),
      0x01 => self.rlc(interrupts, Reg8::C),
      0x11 => self.rl(interrupts, Reg8::C),
      0x21 => self.sla(interrupts, Reg8::C),
      0x31 => self.swap(interrupts, Reg8::C),
      0x02 => self.rlc(interrupts, Reg8::D),
      0x12 => self.rl(interrupts, Reg8::D),
      0x22 => self.sla(interrupts, Reg8::D),
      0x32 => self.swap(interrupts, Reg8::D),
      0x03 => self.rlc(interrupts, Reg8::E),
      0x13 => self.rl(interrupts, Reg8::E),
      0x23 => self.sla(interrupts, Reg8::E),
      0x33 => self.swap(interrupts, Reg8::E),
      0x04 => self.rlc(interrupts, Reg8::H),
      0x14 => self.rl(interrupts, Reg8::H),
      0x24 => self.sla(interrupts, Reg8::H),
      0x34 => self.swap(interrupts, Reg8::H),
      0x05 => self.rlc(interrupts, Reg8::L),
      0x15 => self.rl(interrupts, Reg8::L),
      0x25 => self.sla(interrupts, Reg8::L),
      0x35 => self.swap(interrupts, Reg8::L),
      0x06 => self.rlc(interrupts, Indirect::HL),
      0x16 => self.rl(interrupts, Indirect::HL),
      0x26 => self.sla(interrupts, Indirect::HL),
      0x36 => self.swap(interrupts, Indirect::HL),
      0x07 => self.rlc(interrupts, Reg8::A),
      0x17 => self.rl(interrupts, Reg8::A),
      0x27 => self.sla(interrupts, Reg8::A),
      0x37 => self.swap(interrupts, Reg8::A),
      0x08 => self.rrc(interrupts, Reg8::B),
      0x18 => self.rr(interrupts, Reg8::B),
      0x28 => self.sra(interrupts, Reg8::B),
      0x38 => self.srl(interrupts, Reg8::B),
      0x09 => self.rrc(interrupts, Reg8::C),
      0x19 => self.rr(interrupts, Reg8::C),
      0x29 => self.sra(interrupts, Reg8::C),
      0x39 => self.srl(interrupts, Reg8::C),
      0x0A => self.rrc(interrupts, Reg8::D),
      0x1A => self.rr(interrupts, Reg8::D),
      0x2A => self.sra(interrupts, Reg8::D),
      0x3A => self.srl(interrupts, Reg8::D),
      0x0B => self.rrc(interrupts, Reg8::E),
      0x1B => self.rr(interrupts, Reg8::E),
      0x2B => self.sra(interrupts, Reg8::E),
      0x3B => self.srl(interrupts, Reg8::E),
      0x0C => self.rrc(interrupts, Reg8::H),
      0x1C => self.rr(interrupts, Reg8::H),
      0x2C => self.sra(interrupts, Reg8::H),
      0x3C => self.srl(interrupts, Reg8::H),
      0x0D => self.rrc(interrupts, Reg8::L),
      0x1D => self.rr(interrupts, Reg8::L),
      0x2D => self.sra(interrupts, Reg8::L),
      0x3D => self.srl(interrupts, Reg8::L),
      0x0E => self.rrc(interrupts, Indirect::HL),
      0x1E => self.rr(interrupts, Indirect::HL),
      0x2E => self.sra(interrupts, Indirect::HL),
      0x3E => self.srl(interrupts, Indirect::HL),
      0x0F => self.rrc(interrupts, Reg8::A),
      0x1F => self.rr(interrupts, Reg8::A),
      0x2F => self.sra(interrupts, Reg8::A),
      0x3F => self.srl(interrupts, Reg8::A),
      0x40 => self.bit(interrupts, 0, Reg8::B),
      0x50 => self.bit(interrupts, 2, Reg8::B),
      0x60 => self.bit(interrupts, 4, Reg8::B),
      0x70 => self.bit(interrupts, 6, Reg8::B),
      0x41 => self.bit(interrupts, 0, Reg8::C),
      0x51 => self.bit(interrupts, 2, Reg8::C),
      0x61 => self.bit(interrupts, 4, Reg8::C),
      0x71 => self.bit(interrupts, 6, Reg8::C),
      0x42 => self.bit(interrupts, 0, Reg8::D),
      0x52 => self.bit(interrupts, 2, Reg8::D),
      0x62 => self.bit(interrupts, 4, Reg8::D),
      0x72 => self.bit(interrupts, 6, Reg8::D),
      0x43 => self.bit(interrupts, 0, Reg8::E),
      0x53 => self.bit(interrupts, 2, Reg8::E),
      0x63 => self.bit(interrupts, 4, Reg8::E),
      0x73 => self.bit(interrupts, 6, Reg8::E),
      0x44 => self.bit(interrupts, 0, Reg8::H),
      0x54 => self.bit(interrupts, 2, Reg8::H),
      0x64 => self.bit(interrupts, 4, Reg8::H),
      0x74 => self.bit(interrupts, 6, Reg8::H),
      0x45 => self.bit(interrupts, 0, Reg8::L),
      0x55 => self.bit(interrupts, 2, Reg8::L),
      0x65 => self.bit(interrupts, 4, Reg8::L),
      0x75 => self.bit(interrupts, 6, Reg8::L),
      0x46 => self.bit(interrupts, 0, Indirect::HL),
      0x56 => self.bit(interrupts, 2, Indirect::HL),
      0x66 => self.bit(interrupts, 4, Indirect::HL),
      0x76 => self.bit(interrupts, 6, Indirect::HL),
      0x47 => self.bit(interrupts, 0, Reg8::A),
      0x57 => self.bit(interrupts, 2, Reg8::A),
      0x67 => self.bit(interrupts, 4, Reg8::A),
      0x77 => self.bit(interrupts, 6, Reg8::A),
      0x48 => self.bit(interrupts, 1, Reg8::B),
      0x58 => self.bit(interrupts, 3, Reg8::B),
      0x68 => self.bit(interrupts, 5, Reg8::B),
      0x78 => self.bit(interrupts, 7, Reg8::B),
      0x49 => self.bit(interrupts, 1, Reg8::C),
      0x59 => self.bit(interrupts, 3, Reg8::C),
      0x69 => self.bit(interrupts, 5, Reg8::C),
      0x79 => self.bit(interrupts, 7, Reg8::C),
      0x4A => self.bit(interrupts, 1, Reg8::D),
      0x5A => self.bit(interrupts, 3, Reg8::D),
      0x6A => self.bit(interrupts, 5, Reg8::D),
      0x7A => self.bit(interrupts, 7, Reg8::D),
      0x4B => self.bit(interrupts, 1, Reg8::E),
      0x5B => self.bit(interrupts, 3, Reg8::E),
      0x6B => self.bit(interrupts, 5, Reg8::E),
      0x7B => self.bit(interrupts, 7, Reg8::E),
      0x4C => self.bit(interrupts, 1, Reg8::H),
      0x5C => self.bit(interrupts, 3, Reg8::H),
      0x6C => self.bit(interrupts, 5, Reg8::H),
      0x7C => self.bit(interrupts, 7, Reg8::H),
      0x4D => self.bit(interrupts, 1, Reg8::L),
      0x5D => self.bit(interrupts, 3, Reg8::L),
      0x6D => self.bit(interrupts, 5, Reg8::L),
      0x7D => self.bit(interrupts, 7, Reg8::L),
      0x4E => self.bit(interrupts, 1, Indirect::HL),
      0x5E => self.bit(interrupts, 3, Indirect::HL),
      0x6E => self.bit(interrupts, 5, Indirect::HL),
      0x7E => self.bit(interrupts, 7, Indirect::HL),
      0x4F => self.bit(interrupts, 1, Reg8::A),
      0x5F => self.bit(interrupts, 3, Reg8::A),
      0x6F => self.bit(interrupts, 5, Reg8::A),
      0x7F => self.bit(interrupts, 7, Reg8::A),
      0x80 => self.res(interrupts, 0, Reg8::B),
      0x90 => self.res(interrupts, 2, Reg8::B),
      0xA0 => self.res(interrupts, 4, Reg8::B),
      0xB0 => self.res(interrupts, 6, Reg8::B),
      0x81 => self.res(interrupts, 0, Reg8::C),
      0x91 => self.res(interrupts, 2, Reg8::C),
      0xA1 => self.res(interrupts, 4, Reg8::C),
      0xB1 => self.res(interrupts, 6, Reg8::C),
      0x82 => self.res(interrupts, 0, Reg8::D),
      0x92 => self.res(interrupts, 2, Reg8::D),
      0xA2 => self.res(interrupts, 4, Reg8::D),
      0xB2 => self.res(interrupts, 6, Reg8::D),
      0x83 => self.res(interrupts, 0, Reg8::E),
      0x93 => self.res(interrupts, 2, Reg8::E),
      0xA3 => self.res(interrupts, 4, Reg8::E),
      0xB3 => self.res(interrupts, 6, Reg8::E),
      0x84 => self.res(interrupts, 0, Reg8::H),
      0x94 => self.res(interrupts, 2, Reg8::H),
      0xA4 => self.res(interrupts, 4, Reg8::H),
      0xB4 => self.res(interrupts, 6, Reg8::H),
      0x85 => self.res(interrupts, 0, Reg8::L),
      0x95 => self.res(interrupts, 2, Reg8::L),
      0xA5 => self.res(interrupts, 4, Reg8::L),
      0xB5 => self.res(interrupts, 6, Reg8::L),
      0x86 => self.res(interrupts, 0, Indirect::HL),
      0x96 => self.res(interrupts, 2, Indirect::HL),
      0xA6 => self.res(interrupts, 4, Indirect::HL),
      0xB6 => self.res(interrupts, 6, Indirect::HL),
      0x87 => self.res(interrupts, 0, Reg8::A),
      0x97 => self.res(interrupts, 2, Reg8::A),
      0xA7 => self.res(interrupts, 4, Reg8::A),
      0xB7 => self.res(interrupts, 6, Reg8::A),
      0x88 => self.res(interrupts, 1, Reg8::B),
      0x98 => self.res(interrupts, 3, Reg8::B),
      0xA8 => self.res(interrupts, 5, Reg8::B),
      0xB8 => self.res(interrupts, 7, Reg8::B),
      0x89 => self.res(interrupts, 1, Reg8::C),
      0x99 => self.res(interrupts, 3, Reg8::C),
      0xA9 => self.res(interrupts, 5, Reg8::C),
      0xB9 => self.res(interrupts, 7, Reg8::C),
      0x8A => self.res(interrupts, 1, Reg8::D),
      0x9A => self.res(interrupts, 3, Reg8::D),
      0xAA => self.res(interrupts, 5, Reg8::D),
      0xBA => self.res(interrupts, 7, Reg8::D),
      0x8B => self.res(interrupts, 1, Reg8::E),
      0x9B => self.res(interrupts, 3, Reg8::E),
      0xAB => self.res(interrupts, 5, Reg8::E),
      0xBB => self.res(interrupts, 7, Reg8::E),
      0x8C => self.res(interrupts, 1, Reg8::H),
      0x9C => self.res(interrupts, 3, Reg8::H),
      0xAC => self.res(interrupts, 5, Reg8::H),
      0xBC => self.res(interrupts, 7, Reg8::H),
      0x8D => self.res(interrupts, 1, Reg8::L),
      0x9D => self.res(interrupts, 3, Reg8::L),
      0xAD => self.res(interrupts, 5, Reg8::L),
      0xBD => self.res(interrupts, 7, Reg8::L),
      0x8E => self.res(interrupts, 1, Indirect::HL),
      0x9E => self.res(interrupts, 3, Indirect::HL),
      0xAE => self.res(interrupts, 5, Indirect::HL),
      0xBE => self.res(interrupts, 7, Indirect::HL),
      0x8F => self.res(interrupts, 1, Reg8::A),
      0x9F => self.res(interrupts, 3, Reg8::A),
      0xAF => self.res(interrupts, 5, Reg8::A),
      0xBF => self.res(interrupts, 7, Reg8::A),
      0xC0 => self.set(interrupts, 0, Reg8::B),
      0xD0 => self.set(interrupts, 2, Reg8::B),
      0xE0 => self.set(interrupts, 4, Reg8::B),
      0xF0 => self.set(interrupts, 6, Reg8::B),
      0xC1 => self.set(interrupts, 0, Reg8::C),
      0xD1 => self.set(interrupts, 2, Reg8::C),
      0xE1 => self.set(interrupts, 4, Reg8::C),
      0xF1 => self.set(interrupts, 6, Reg8::C),
      0xC2 => self.set(interrupts, 0, Reg8::D),
      0xD2 => self.set(interrupts, 2, Reg8::D),
      0xE2 => self.set(interrupts, 4, Reg8::D),
      0xF2 => self.set(interrupts, 6, Reg8::D),
      0xC3 => self.set(interrupts, 0, Reg8::E),
      0xD3 => self.set(interrupts, 2, Reg8::E),
      0xE3 => self.set(interrupts, 4, Reg8::E),
      0xF3 => self.set(interrupts, 6, Reg8::E),
      0xC4 => self.set(interrupts, 0, Reg8::H),
      0xD4 => self.set(interrupts, 2, Reg8::H),
      0xE4 => self.set(interrupts, 4, Reg8::H),
      0xF4 => self.set(interrupts, 6, Reg8::H),
      0xC5 => self.set(interrupts, 0, Reg8::L),
      0xD5 => self.set(interrupts, 2, Reg8::L),
      0xE5 => self.set(interrupts, 4, Reg8::L),
      0xF5 => self.set(interrupts, 6, Reg8::L),
      0xC6 => self.set(interrupts, 0, Indirect::HL),
      0xD6 => self.set(interrupts, 2, Indirect::HL),
      0xE6 => self.set(interrupts, 4, Indirect::HL),
      0xF6 => self.set(interrupts, 6, Indirect::HL),
      0xC7 => self.set(interrupts, 0, Reg8::A),
      0xD7 => self.set(interrupts, 2, Reg8::A),
      0xE7 => self.set(interrupts, 4, Reg8::A),
      0xF7 => self.set(interrupts, 6, Reg8::A),
      0xC8 => self.set(interrupts, 1, Reg8::B),
      0xD8 => self.set(interrupts, 3, Reg8::B),
      0xE8 => self.set(interrupts, 5, Reg8::B),
      0xF8 => self.set(interrupts, 7, Reg8::B),
      0xC9 => self.set(interrupts, 1, Reg8::C),
      0xD9 => self.set(interrupts, 3, Reg8::C),
      0xE9 => self.set(interrupts, 5, Reg8::C),
      0xF9 => self.set(interrupts, 7, Reg8::C),
      0xCA => self.set(interrupts, 1, Reg8::D),
      0xDA => self.set(interrupts, 3, Reg8::D),
      0xEA => self.set(interrupts, 5, Reg8::D),
      0xFA => self.set(interrupts, 7, Reg8::D),
      0xCB => self.set(interrupts, 1, Reg8::E),
      0xDB => self.set(interrupts, 3, Reg8::E),
      0xEB => self.set(interrupts, 5, Reg8::E),
      0xFB => self.set(interrupts, 7, Reg8::E),
      0xCC => self.set(interrupts, 1, Reg8::H),
      0xDC => self.set(interrupts, 3, Reg8::H),
      0xEC => self.set(interrupts, 5, Reg8::H),
      0xFC => self.set(interrupts, 7, Reg8::H),
      0xCD => self.set(interrupts, 1, Reg8::L),
      0xDD => self.set(interrupts, 3, Reg8::L),
      0xED => self.set(interrupts, 5, Reg8::L),
      0xFD => self.set(interrupts, 7, Reg8::L),
      0xCE => self.set(interrupts, 1, Indirect::HL),
      0xDE => self.set(interrupts, 3, Indirect::HL),
      0xEE => self.set(interrupts, 5, Indirect::HL),
      0xFE => self.set(interrupts, 7, Indirect::HL),
      0xCF => self.set(interrupts, 1, Reg8::A),
      0xDF => self.set(interrupts, 3, Reg8::A),
      0xEF => self.set(interrupts, 5, Reg8::A),
      0xFF => self.set(interrupts, 7, Reg8::A),
    }
  }
  fn interrupt_dispatch(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   : {
        self.ime = false;
        self.instruction_step.to_next();
      },
      Two   : {
        self.val16 = self.regs.pc;
        self.instruction_step.to_next();
      },
      Three : {
        let [lo, hi] = u16::to_le_bytes(self.val16);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        (self.bus.write)(interrupts, self.regs.sp, hi);
        self.val8 = lo;
        self.instruction_step.to_next();
      },
      Four  : {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        (self.bus.write)(interrupts, self.regs.sp, self.val8);
        let interrupt: u8 = isolate_rightmost_onebit(interrupts.get_interrupt()); // get highest priority interrupt
        interrupts.ack_interrupt(interrupt);
        self.regs.pc = match interrupt {
          interrupts::VBLANK => 0x0040,
          interrupts::STAT => 0x0048,
          interrupts::TIMER => 0x0050,
          interrupts::SERIAL => 0x0058,
          interrupts::JOYPAD => 0x0060,
          _ => panic!("Invalid interrupt: {:02x}", interrupt),
        };
        self.instruction_step.to_next();
      },
      Five  : self.prefetch(interrupts),
    );
  }
  // 8-bit operations
  fn ld<D: Copy, S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, dst: D, src: S)
  where Self: IO8<D> + IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > self.write8(interrupts, dst, self.val8),
      Three > self.prefetch(interrupts),
    );
  }
  fn add<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let (result, carry) = self.regs.a.overflowing_add(self.val8);
        let half_carry = (self.regs.a & 0x0f).checked_add(self.val8 | 0xf0).is_none();
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(half_carry);
        self.regs.set_cf(carry);
        self.regs.a = result;
        self.prefetch(interrupts);
      },
    );
  }
  fn adc<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let cy = self.regs.cf() as u8;
        let result = self.regs.a.wrapping_add(self.val8).wrapping_add(cy);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(
          (self.regs.a & 0xf) + (self.val8 & 0xf) + cy > 0xf
        );
        self.regs.set_cf(
          self.regs.a as u16 + self.val8 as u16 + cy as u16 > 0xff
        );
        self.regs.a = result;
        self.prefetch(interrupts);
      },
    );
  }
  fn sub<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.regs.a = self.alu_sub(self.val8, false);
        self.prefetch(interrupts);
      },
    );
  }
  fn sbc<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.regs.a = self.alu_sub(self.val8, self.regs.cf());
        self.prefetch(interrupts);
      },
    );
  }
  fn cp<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.alu_sub(self.val8, false);
        self.prefetch(interrupts);
      },
    );
  }
  fn and<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.regs.a &= self.val8;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.prefetch(interrupts);
      },
    );
  }
  fn or<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.regs.a |= self.val8;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.prefetch(interrupts);
      },
    );
  }
  fn xor<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.regs.a ^= self.val8;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.prefetch(interrupts);
      },
    );
  }
  fn inc<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let new_val = self.val8.wrapping_add(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(self.val8 & 0xf == 0xf);
        self.val8 = new_val;
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn dec<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let new_val = self.val8.wrapping_sub(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(self.val8 & 0xf == 0);
        self.val8 = new_val;
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn rlca(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.a = self.alu_rlc(self.regs.a);
    self.regs.set_zf(false);
    self.prefetch(interrupts);
  }
  fn rla(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.a = self.alu_rl(self.regs.a);
    self.regs.set_zf(false);
    self.prefetch(interrupts);
  }
  fn rrca(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.a = self.alu_rrc(self.regs.a);
    self.regs.set_zf(false);
    self.prefetch(interrupts);
  }
  fn rra(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.a = self.alu_rr(self.regs.a);
    self.regs.set_zf(false);
    self.prefetch(interrupts);
  }
  fn rlc<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 = self.alu_rlc(self.val8);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn rl<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 = self.alu_rl(self.val8);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn rrc<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 = self.alu_rrc(self.val8);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn rr<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 = self.alu_rr(self.val8);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn sla<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let co = self.val8 & 0x80;
        self.val8 = self.val8 << 1;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co > 0);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn sra<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let co = self.val8 & 0x01;
        let hi = self.val8 & 0x80;
        self.val8 = (self.val8 >> 1) | hi;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co > 0);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn srl<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        let co = self.val8 & 0x01;
        self.val8 = self.val8 >> 1;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co != 0);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn swap<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 = (self.val8 >> 4) | (self.val8 << 4);
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  >self.prefetch(interrupts),
    );
  }
  fn bit<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, bit: usize, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 &= 1 << bit;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.prefetch(interrupts);
      },
    );
  }
  fn set<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, bit: usize, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 |= 1 << bit;
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn res<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, bit: usize, src: S)
  where Self: IO8<S> {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, src).unwrap_or_default(),
      Two   > {
        self.val8 &= !(1 << bit);
        self.instruction_step.to_next();
      },
      Three > self.write8(interrupts, src, self.val8),
      Four  > self.prefetch(interrupts),
    );
  }
  fn jp(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, Imm16).unwrap_or_default(),
      Two   : {
        self.regs.pc = self.val16;
        self.instruction_step.to_next();
      },
      Three > self.prefetch(interrupts),
    );
  }
  fn jp_hl(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.pc = self.regs.hl();
    self.prefetch(interrupts);
  }
  fn jr(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, Imm8).unwrap_or_default(),
      Two   : {
        self.regs.pc = self.regs.pc.wrapping_add((self.val8 as i8) as u16);
        self.instruction_step.to_next();
      },
      Three > self.prefetch(interrupts),
    );
  }
  fn call(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, Imm16).unwrap_or_default(),
      Two   > self.push16(interrupts, self.regs.pc),
      Three > {
        self.regs.pc = self.val16;
        self.prefetch(interrupts);
      },
    );
  }
  fn ret(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   > self.val16 = self.pop16(interrupts).unwrap_or_default(),
      Two   : {
        self.regs.pc = self.val16;
        self.instruction_step.to_next();
      },
      Three > self.prefetch(interrupts),
    );
  }
  fn reti(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   > {
        self.ime = true;
        self.instruction_step.to_next();
      },
      Two   > self.val16 = self.pop16(interrupts).unwrap_or_default(),
      Three : {
        self.regs.pc = self.val16;
        self.instruction_step.to_next();
      },
      Four  > self.prefetch(interrupts),
    );
  }
  fn jp_cc(&mut self, interrupts: &mut interrupts::Interrupts, cond: Cond) {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, Imm16).unwrap_or_default(),
      Two   : {
        if self.check_cond(cond) {
          self.regs.pc = self.val16;
          self.instruction_step.to_next();
        } else {
          self.prefetch(interrupts);
        }
      },
      Three > self.prefetch(interrupts),
    );
  }
  fn jr_cc(&mut self, interrupts: &mut interrupts::Interrupts, cond: Cond) {
    step!(self.instruction_step,
      One   > self.val8 = self.read8(interrupts, Imm8).unwrap_or_default(),
      Two   : {
        if self.check_cond(cond) {
          self.regs.pc = self.regs.pc.wrapping_add((self.val8 as i8) as u16);
          self.instruction_step.to_next();
        } else {
          self.prefetch(interrupts);
        }
      },
      Three > self.prefetch(interrupts),
    );
  }
  fn call_cc(&mut self, interrupts: &mut interrupts::Interrupts, cond: Cond) {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, Imm16).unwrap_or_default(),
      Two   > {
        if self.check_cond(cond) {
          self.instruction_step.to_next();
        } else {
          self.prefetch(interrupts);
        }
      },
      Three > self.push16(interrupts, self.regs.pc),
      Four  > {
        self.regs.pc = self.val16;
        self.prefetch(interrupts);
      },
    );
  }
  fn ret_cc(&mut self, interrupts: &mut interrupts::Interrupts, cond: Cond) {
    step!(self.instruction_step,
      One   : self.instruction_step.to_next(),
      Two   > {
        if self.check_cond(cond) {
          self.instruction_step.to_next();
        } else {
          self.prefetch(interrupts);
        }
      },
      Three > self.val16 = self.pop16(interrupts).unwrap_or_default(),
      Four  : {
        self.regs.pc = self.val16;
        self.instruction_step.to_next();
      },
      Five  > self.prefetch(interrupts),
    );
  }
  fn rst(&mut self, interrupts: &mut interrupts::Interrupts, addr: u8) {
    step!(self.instruction_step,
      One   > self.push16(interrupts, self.regs.pc),
      Two   > {
        self.regs.pc = addr as u16;
        self.prefetch(interrupts);
      },
    );
  }
  fn halt(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   : self.instruction_step.to_next(),
      Two   : {
        if interrupts.get_interrupt() > 0 {
          self.instruction_step = Step::One(0);
          if self.ime {
            self.state = State::InterruptDispatch;
          } else {
            // This causes halt bug. (https://gbdev.io/pandocs/halt.html#halt-bug)
            self.opcode = (self.bus.read)(interrupts, self.regs.pc);
            // self.prefetch(interrupts);
            // self.decode_exec_fetch_cycle(interrupts);
          }
        } else {
          self.instruction_step.to_next();
        }
      },
      Three : {
        self.state = State::Halt;
        self.instruction_step = Step::One(0);
      },
    );
  }
  fn stop(&mut self) {
    panic!("STOP");
  }
  fn di(&mut self, interrupts: &interrupts::Interrupts) {
    self.ime = false;
    self.prefetch(interrupts);
  }
  fn ei(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.prefetch(interrupts);
    self.ime = true;
  }
  fn ccf(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(!self.regs.cf());
    self.prefetch(interrupts);
  }
  fn scf(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(true);
    self.prefetch(interrupts);
  }
  fn nop(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.prefetch(interrupts);
  }
  fn daa(&mut self, interrupts: &mut interrupts::Interrupts) {
    // DAA table in page 110 of the official "Game Boy Programming Manual"
    let mut carry = false;
    if !self.regs.nf() {
      if self.regs.cf() || self.regs.a > 0x99 {
        self.regs.a = self.regs.a.wrapping_add(0x60);
        carry = true;
      }
      if self.regs.hf() || self.regs.a & 0x0f > 0x09 {
        self.regs.a = self.regs.a.wrapping_add(0x06);
      }
    } else if self.regs.cf() {
      carry = true;
      self.regs.a = self.regs.a.wrapping_add(
        if self.regs.hf() { 0x9a } else { 0xa0 }
      );
    } else if self.regs.hf() {
      self.regs.a = self.regs.a.wrapping_add(0xfa);
    }

    self.regs.set_zf(self.regs.a == 0);
    self.regs.set_hf(false);
    self.regs.set_cf(carry);
    self.prefetch(interrupts);
  }
  fn cpl(&mut self, interrupts: &mut interrupts::Interrupts) {
    self.regs.a = !self.regs.a;
    self.regs.set_nf(true);
    self.regs.set_hf(true);
    self.prefetch(interrupts);
  }
  // 16-bit operations
  fn ld16<D: Copy, S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, dst: D, src: S)
  where Self: IO16<D> + IO16<S> {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, src).unwrap_or_default(),
      Two   > self.write16(interrupts, dst, self.val16),
      Three > self.prefetch(interrupts),
    );
  }
  fn ld_sp_hl(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   : {
        self.regs.sp = self.regs.hl();
        self.instruction_step.to_next();
      },
      Two   : self.prefetch(interrupts),
    );
  }
  fn ld_hl_sp_e(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   : {
        self.val16 = self.read_imm8(interrupts) as i8 as u16;
        self.instruction_step.to_next();
      },
      Two   : {
        let val = self.regs.sp.wrapping_add(self.val16);
        self.write_r16(Reg16::HL, val);
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf(check_add_carry(3, self.regs.sp, self.val16));
        self.regs.set_cf(check_add_carry(7, self.regs.sp, self.val16));
        self.instruction_step.to_next();
      },
      Three : self.prefetch(interrupts),
    );
  }
  fn push_r16(&mut self, interrupts: &mut interrupts::Interrupts, src: Reg16) {
    step!(self.instruction_step,
      One   > {
        self.val16 = self.read_r16(src);
        self.instruction_step.to_next();
      },
      Two   > self.push16(interrupts, self.val16),
      Three > self.prefetch(interrupts),
    );
  }
  fn pop_r16(&mut self, interrupts: &mut interrupts::Interrupts, dst: Reg16) {
    step!(self.instruction_step,
      One   > self.val16 = self.pop16(interrupts).unwrap_or_default(),
      Two   > {
        self.write_r16(dst, self.val16);
        self.prefetch(interrupts);
      },
    );
  }
  fn add_hl_r16(&mut self, interrupts: &mut interrupts::Interrupts, src: Reg16) {
    step!(self.instruction_step,
      One   : {
        let hl = self.regs.hl();
        let val = self.read_r16(src);
        self.regs.set_nf(false);
        self.regs.set_hf(check_add_carry(11, hl, val));
        self.regs.set_cf(hl > 0xffff - val);
        self.write_r16(Reg16::HL, hl.wrapping_add(val));
        self.instruction_step.to_next();
      },
      Two   : self.prefetch(interrupts),
    );
  }
  fn add_sp_e(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   : {
        let val = self.read_imm8(interrupts) as i8 as i16 as u16;
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf(check_add_carry(3, self.regs.sp, val));
        self.regs.set_cf(check_add_carry(7, self.regs.sp, val));
        self.regs.sp = self.regs.sp.wrapping_add(val);
        self.instruction_step.to_next();
      },
      Two   : self.instruction_step.to_next(),
      Three : self.instruction_step.to_next(),
      Four  : self.prefetch(interrupts),
    );
  }
  fn inc16<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO16<S> {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, src).unwrap_or_default().wrapping_add(1),
      Two   : self.write16(interrupts, src, self.val16),
      Three > self.prefetch(interrupts),
    );
  }
  fn dec16<S: Copy>(&mut self, interrupts: &mut interrupts::Interrupts, src: S)
  where Self: IO16<S> {
    step!(self.instruction_step,
      One   > self.val16 = self.read16(interrupts, src).unwrap_or_default().wrapping_sub(1),
      Two   : self.write16(interrupts, src, self.val16),
      Three > self.prefetch(interrupts),
    );
  }
  fn undefined(&mut self) {
    panic!("Undefined opcode {:02x}", self.opcode);
  }
  fn cb_prefix(&mut self, interrupts: &mut interrupts::Interrupts) {
    step!(self.instruction_step,
      One   : self.instruction_step.to_next(),
      Two   : {
        self.opcode = self.read_imm8(interrupts);
        self.instruction_step = Step::One(0);
        self.cb = true;
        self.cb_decode_exec_fetch_cycle(interrupts);
      },
    );
  }
}
