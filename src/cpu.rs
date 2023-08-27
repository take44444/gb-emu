use crate::register;
use crate::interrupts;
use crate::peripherals;

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
pub enum Cond {
  NZ,
  Z,
  NC,
  C,
}

#[derive(Clone, Copy, Debug)]
pub enum Reg8 {
  A,
  B,
  C,
  D,
  E,
  H,
  L,
}

#[derive(Clone, Copy, Debug)]
pub enum Reg16 {
  AF,
  BC,
  DE,
  HL,
  SP,
}

#[derive(Clone, Copy, Debug)]
pub enum Indirect {
  BC,
  DE,
  HL,
  CFF,
  HLD,
  HLI,
}

#[derive(Clone, Copy, Debug)]
pub enum Direct {
  D,
  DFF,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
  Running,
  InterruptDispatch,
  Halt,
}

pub struct Cpu {
  cb: bool,
  state: State,
  regs: register::Registers,
  ime: bool,
  opcode: u8,
  command_cycle: u8,
  val8: u8,
  val16: u16,
}

impl Cpu {
  pub fn new() -> Self {
    Self {
      cb: false,
      state: State::Running,
      regs: register::Registers::new(),
      ime: false,
      opcode: 0x00,
      command_cycle: 0,
      val8: 0,
      val16: 0,
    }
  }

  fn prefetch(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    self.opcode = peripherals.read(interrupts, self.regs.pc);
    let interrupt = interrupts.get_interrupt();
    if self.ime && interrupt != 0 {
      self.state = State::InterruptDispatch;
    } else {
      self.regs.pc = self.regs.pc.wrapping_add(1);
      self.state = State::Running;
    }
    self.command_cycle = 0;
    self.cb = false;
  }

  // read absolute addr specified by pc register
  fn read_imm8(&mut self, interrupts: &interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) -> u8 {
    let ret = peripherals.read(interrupts, self.regs.pc);
    self.regs.pc = self.regs.pc.wrapping_add(1);
    ret
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

  // read data from 8bit register
  fn read_r8(&self, src: Reg8) -> u8 {
    match src {
      Reg8::A => self.regs.a,
      Reg8::B => self.regs.b,
      Reg8::C => self.regs.c,
      Reg8::D => self.regs.d,
      Reg8::E => self.regs.e,
      Reg8::H => self.regs.h,
      Reg8::L => self.regs.l,
    }
  }
  // write data to 8bit register
  fn write_r8(&mut self, dst: Reg8, val: u8) {
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
  // read data from 16bit register
  fn read_r16(&self, src: Reg16) -> u16 {
    match src {
      Reg16::AF => self.regs.af(),
      Reg16::BC => self.regs.bc(),
      Reg16::DE => self.regs.de(),
      Reg16::HL => self.regs.hl(),
      Reg16::SP => self.regs.sp,
    }
  }
  // write data to 16bit register
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

  // read absolute addr specified by 16bit reg
  fn read_indirect(&mut self, interrupts: &interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Indirect) -> u8 {
    match src {
      Indirect::BC => peripherals.read(interrupts, self.regs.bc()),
      Indirect::DE => peripherals.read(interrupts, self.regs.de()),
      Indirect::HL => peripherals.read(interrupts, self.regs.hl()),
      Indirect::CFF => peripherals.read(interrupts, 0xff00 | (self.regs.c as u16)),
      Indirect::HLD => {
        let addr = self.regs.hl();
        self.write_r16(Reg16::HL, addr.wrapping_sub(1));
        peripherals.read(interrupts, addr)
      },
      Indirect::HLI => {
        let addr = self.regs.hl();
        self.write_r16(Reg16::HL, addr.wrapping_add(1));
        peripherals.read(interrupts, addr)
      },
    }
  }
  // write data to absolute addr specified by 16bit reg
  fn write_indirect(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Indirect, val: u8) {
    match dst {
      Indirect::BC => peripherals.write(interrupts, self.regs.bc(), val),
      Indirect::DE => peripherals.write(interrupts, self.regs.de(), val),
      Indirect::HL => peripherals.write(interrupts, self.regs.hl(), val),
      Indirect::CFF => peripherals.write(interrupts, 0xff00 | (self.regs.c as u16), val),
      Indirect::HLD => {
        let addr = self.regs.hl();
        peripherals.write(interrupts, addr, val);
        self.write_r16(Reg16::HL, addr.wrapping_sub(1));
      },
      Indirect::HLI => {
        let addr = self.regs.hl();
        peripherals.write(interrupts, addr, val);
        self.write_r16(Reg16::HL, addr.wrapping_add(1));
      },
    }
  }

  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.state {
      State::Running => self.decode_exec_fetch_cycle(interrupts, peripherals),
      State::InterruptDispatch => self.interrupt_dispatch(interrupts, peripherals),
      State::Halt => {
        if interrupts.get_interrupt() != 0 {
          self.prefetch(interrupts, peripherals);
        }
      }
    }
  }
  // https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
  fn decode_exec_fetch_cycle(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    if self.cb {
      self.cb_decode_exec_fetch_cycle(interrupts, peripherals);
      return;
    }
    match self.opcode {
      0x00 => self.nop(interrupts, peripherals),
      0x10 => self.stop(),
      0x20 => self.jr_cc_imm8(interrupts, peripherals, Cond::NZ),
      0x30 => self.jr_cc_imm8(interrupts, peripherals, Cond::NC),
      0x01 => self.ld_r16_imm16(interrupts, peripherals, Reg16::BC),
      0x11 => self.ld_r16_imm16(interrupts, peripherals, Reg16::DE),
      0x21 => self.ld_r16_imm16(interrupts, peripherals, Reg16::HL),
      0x31 => self.ld_r16_imm16(interrupts, peripherals, Reg16::SP),
      0x02 => self.ld_indirect_r8(interrupts, peripherals, Indirect::BC, Reg8::A),
      0x12 => self.ld_indirect_r8(interrupts, peripherals, Indirect::DE, Reg8::A),
      0x22 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HLI, Reg8::A),
      0x32 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HLD, Reg8::A),
      0x03 => self.inc_r16(interrupts, peripherals, Reg16::BC),
      0x13 => self.inc_r16(interrupts, peripherals, Reg16::DE),
      0x23 => self.inc_r16(interrupts, peripherals, Reg16::HL),
      0x33 => self.inc_r16(interrupts, peripherals, Reg16::SP),
      0x04 => self.inc_r8(interrupts, peripherals, Reg8::B),
      0x14 => self.inc_r8(interrupts, peripherals, Reg8::D),
      0x24 => self.inc_r8(interrupts, peripherals, Reg8::H),
      0x34 => self.inc_hl(interrupts, peripherals),
      0x05 => self.dec_r8(interrupts, peripherals, Reg8::B),
      0x15 => self.dec_r8(interrupts, peripherals, Reg8::D),
      0x25 => self.dec_r8(interrupts, peripherals, Reg8::H),
      0x35 => self.dec_hl(interrupts, peripherals),
      0x06 => self.ld_r8_imm8(interrupts, peripherals, Reg8::B),
      0x16 => self.ld_r8_imm8(interrupts, peripherals, Reg8::D),
      0x26 => self.ld_r8_imm8(interrupts, peripherals, Reg8::H),
      0x36 => self.ld_indirect_imm8(interrupts, peripherals, Indirect::HL),
      0x07 => self.rlca(interrupts, peripherals),
      0x17 => self.rla(interrupts, peripherals),
      0x27 => self.daa(interrupts, peripherals),
      0x37 => self.scf(interrupts, peripherals),
      0x08 => self.ld_direct_sp(interrupts, peripherals),
      0x18 => self.jr_imm8(interrupts, peripherals),
      0x28 => self.jr_cc_imm8(interrupts, peripherals, Cond::Z),
      0x38 => self.jr_cc_imm8(interrupts, peripherals, Cond::C),
      0x09 => self.add_hl_r16(interrupts, peripherals, Reg16::BC),
      0x19 => self.add_hl_r16(interrupts, peripherals, Reg16::DE),
      0x29 => self.add_hl_r16(interrupts, peripherals, Reg16::HL),
      0x39 => self.add_hl_r16(interrupts, peripherals, Reg16::SP),
      0x0A => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::BC),
      0x1A => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::DE),
      0x2A => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::HLI),
      0x3A => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::HLD),
      0x0B => self.dec_r16(interrupts, peripherals, Reg16::BC),
      0x1B => self.dec_r16(interrupts, peripherals, Reg16::DE),
      0x2B => self.dec_r16(interrupts, peripherals, Reg16::HL),
      0x3B => self.dec_r16(interrupts, peripherals, Reg16::SP),
      0x0C => self.inc_r8(interrupts, peripherals, Reg8::C),
      0x1C => self.inc_r8(interrupts, peripherals, Reg8::E),
      0x2C => self.inc_r8(interrupts, peripherals, Reg8::L),
      0x3C => self.inc_r8(interrupts, peripherals, Reg8::A),
      0x0D => self.dec_r8(interrupts, peripherals, Reg8::C),
      0x1D => self.dec_r8(interrupts, peripherals, Reg8::E),
      0x2D => self.dec_r8(interrupts, peripherals, Reg8::L),
      0x3D => self.dec_r8(interrupts, peripherals, Reg8::A),
      0x0E => self.ld_r8_imm8(interrupts, peripherals, Reg8::C),
      0x1E => self.ld_r8_imm8(interrupts, peripherals, Reg8::E),
      0x2E => self.ld_r8_imm8(interrupts, peripherals, Reg8::L),
      0x3E => self.ld_r8_imm8(interrupts, peripherals, Reg8::A),
      0x0F => self.rrca(interrupts, peripherals),
      0x1F => self.rra(interrupts, peripherals),
      0x2F => self.cpl(interrupts, peripherals),
      0x3F => self.ccf(interrupts, peripherals),
      0x40 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::B),
      0x50 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::B),
      0x60 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::B),
      0x70 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::B),
      0x41 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::C),
      0x51 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::C),
      0x61 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::C),
      0x71 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::C),
      0x42 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::D),
      0x52 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::D),
      0x62 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::D),
      0x72 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::D),
      0x43 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::E),
      0x53 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::E),
      0x63 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::E),
      0x73 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::E),
      0x44 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::H),
      0x54 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::H),
      0x64 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::H),
      0x74 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::H),
      0x45 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::L),
      0x55 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::L),
      0x65 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::L),
      0x75 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::L),
      0x46 => self.ld_r8_indirect(interrupts, peripherals, Reg8::B, Indirect::HL),
      0x56 => self.ld_r8_indirect(interrupts, peripherals, Reg8::D, Indirect::HL),
      0x66 => self.ld_r8_indirect(interrupts, peripherals, Reg8::H, Indirect::HL),
      0x76 => self.halt(interrupts, peripherals),
      0x47 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::A),
      0x57 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::A),
      0x67 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::A),
      0x77 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::A),
      0x48 => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::B),
      0x58 => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::B),
      0x68 => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::B),
      0x78 => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::B),
      0x49 => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::C),
      0x59 => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::C),
      0x69 => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::C),
      0x79 => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::C),
      0x4A => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::D),
      0x5A => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::D),
      0x6A => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::D),
      0x7A => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::D),
      0x4B => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::E),
      0x5B => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::E),
      0x6B => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::E),
      0x7B => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::E),
      0x4C => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::H),
      0x5C => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::H),
      0x6C => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::H),
      0x7C => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::H),
      0x4D => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::L),
      0x5D => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::L),
      0x6D => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::L),
      0x7D => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::L),
      0x4E => self.ld_r8_indirect(interrupts, peripherals, Reg8::C, Indirect::HL),
      0x5E => self.ld_r8_indirect(interrupts, peripherals, Reg8::E, Indirect::HL),
      0x6E => self.ld_r8_indirect(interrupts, peripherals, Reg8::L, Indirect::HL),
      0x7E => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::HL),
      0x4F => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::A),
      0x5F => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::A),
      0x6F => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::A),
      0x7F => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::A),
      0x80 => self.add_r8(interrupts, peripherals, Reg8::B),
      0x90 => self.sub_r8(interrupts, peripherals, Reg8::B),
      0xA0 => self.and_r8(interrupts, peripherals, Reg8::B),
      0xB0 => self.or_r8(interrupts, peripherals, Reg8::B),
      0x81 => self.add_r8(interrupts, peripherals, Reg8::C),
      0x91 => self.sub_r8(interrupts, peripherals, Reg8::C),
      0xA1 => self.and_r8(interrupts, peripherals, Reg8::C),
      0xB1 => self.or_r8(interrupts, peripherals, Reg8::C),
      0x82 => self.add_r8(interrupts, peripherals, Reg8::D),
      0x92 => self.sub_r8(interrupts, peripherals, Reg8::D),
      0xA2 => self.and_r8(interrupts, peripherals, Reg8::D),
      0xB2 => self.or_r8(interrupts, peripherals, Reg8::D),
      0x83 => self.add_r8(interrupts, peripherals, Reg8::E),
      0x93 => self.sub_r8(interrupts, peripherals, Reg8::E),
      0xA3 => self.and_r8(interrupts, peripherals, Reg8::E),
      0xB3 => self.or_r8(interrupts, peripherals, Reg8::E),
      0x84 => self.add_r8(interrupts, peripherals, Reg8::H),
      0x94 => self.sub_r8(interrupts, peripherals, Reg8::H),
      0xA4 => self.and_r8(interrupts, peripherals, Reg8::H),
      0xB4 => self.or_r8(interrupts, peripherals, Reg8::H),
      0x85 => self.add_r8(interrupts, peripherals, Reg8::L),
      0x95 => self.sub_r8(interrupts, peripherals, Reg8::L),
      0xA5 => self.and_r8(interrupts, peripherals, Reg8::L),
      0xB5 => self.or_r8(interrupts, peripherals, Reg8::L),
      0x86 => self.add_hl(interrupts, peripherals),
      0x96 => self.sub_hl(interrupts, peripherals),
      0xA6 => self.and_hl(interrupts, peripherals),
      0xB6 => self.or_hl(interrupts, peripherals),
      0x87 => self.add_r8(interrupts, peripherals, Reg8::A),
      0x97 => self.sub_r8(interrupts, peripherals, Reg8::A),
      0xA7 => self.and_r8(interrupts, peripherals, Reg8::A),
      0xB7 => self.or_r8(interrupts, peripherals, Reg8::A),
      0x88 => self.adc_r8(interrupts, peripherals, Reg8::B),
      0x98 => self.sbc_r8(interrupts, peripherals, Reg8::B),
      0xA8 => self.xor_r8(interrupts, peripherals, Reg8::B),
      0xB8 => self.cp_r8(interrupts, peripherals, Reg8::B),
      0x89 => self.adc_r8(interrupts, peripherals, Reg8::C),
      0x99 => self.sbc_r8(interrupts, peripherals, Reg8::C),
      0xA9 => self.xor_r8(interrupts, peripherals, Reg8::C),
      0xB9 => self.cp_r8(interrupts, peripherals, Reg8::C),
      0x8A => self.adc_r8(interrupts, peripherals, Reg8::D),
      0x9A => self.sbc_r8(interrupts, peripherals, Reg8::D),
      0xAA => self.xor_r8(interrupts, peripherals, Reg8::D),
      0xBA => self.cp_r8(interrupts, peripherals, Reg8::D),
      0x8B => self.adc_r8(interrupts, peripherals, Reg8::E),
      0x9B => self.sbc_r8(interrupts, peripherals, Reg8::E),
      0xAB => self.xor_r8(interrupts, peripherals, Reg8::E),
      0xBB => self.cp_r8(interrupts, peripherals, Reg8::E),
      0x8C => self.adc_r8(interrupts, peripherals, Reg8::H),
      0x9C => self.sbc_r8(interrupts, peripherals, Reg8::H),
      0xAC => self.xor_r8(interrupts, peripherals, Reg8::H),
      0xBC => self.cp_r8(interrupts, peripherals, Reg8::H),
      0x8D => self.adc_r8(interrupts, peripherals, Reg8::L),
      0x9D => self.sbc_r8(interrupts, peripherals, Reg8::L),
      0xAD => self.xor_r8(interrupts, peripherals, Reg8::L),
      0xBD => self.cp_r8(interrupts, peripherals, Reg8::L),
      0x8E => self.adc_hl(interrupts, peripherals),
      0x9E => self.sbc_hl(interrupts, peripherals),
      0xAE => self.xor_hl(interrupts, peripherals),
      0xBE => self.cp_hl(interrupts, peripherals),
      0x8F => self.adc_r8(interrupts, peripherals, Reg8::A),
      0x9F => self.sbc_r8(interrupts, peripherals, Reg8::A),
      0xAF => self.xor_r8(interrupts, peripherals, Reg8::A),
      0xBF => self.cp_r8(interrupts, peripherals, Reg8::A),
      0xC0 => self.ret_cc(interrupts, peripherals, Cond::NZ),
      0xD0 => self.ret_cc(interrupts, peripherals, Cond::NC),
      0xE0 => self.ld_direct_r8(interrupts, peripherals, Direct::DFF, Reg8::A),
      0xF0 => self.ld_r8_direct(interrupts, peripherals, Reg8::A, Direct::DFF),
      0xC1 => self.pop_r16(interrupts, peripherals, Reg16::BC),
      0xD1 => self.pop_r16(interrupts, peripherals, Reg16::DE),
      0xE1 => self.pop_r16(interrupts, peripherals, Reg16::HL),
      0xF1 => self.pop_r16(interrupts, peripherals, Reg16::AF),
      0xC2 => self.jp_cc_imm16(interrupts, peripherals, Cond::NZ),
      0xD2 => self.jp_cc_imm16(interrupts, peripherals, Cond::NC),
      0xE2 => self.ld_indirect_r8(interrupts, peripherals, Indirect::CFF, Reg8::A),
      0xF2 => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::CFF),
      0xC3 => self.jp_imm16(interrupts, peripherals),
      0xD3 => self.undefined(),
      0xE3 => self.undefined(),
      0xF3 => self.di(interrupts, peripherals),
      0xC4 => self.call_cc_imm16(interrupts, peripherals, Cond::NZ),
      0xD4 => self.call_cc_imm16(interrupts, peripherals, Cond::NC),
      0xE4 => self.undefined(),
      0xF4 => self.undefined(),
      0xC5 => self.push_r16(interrupts, peripherals, Reg16::BC),
      0xD5 => self.push_r16(interrupts, peripherals, Reg16::DE),
      0xE5 => self.push_r16(interrupts, peripherals, Reg16::HL),
      0xF5 => self.push_r16(interrupts, peripherals, Reg16::AF),
      0xC6 => self.add_imm8(interrupts, peripherals),
      0xD6 => self.sub_imm8(interrupts, peripherals),
      0xE6 => self.and_imm8(interrupts, peripherals),
      0xF6 => self.or_imm8(interrupts, peripherals),
      0xC7 => self.rst(interrupts, peripherals, 0x00),
      0xD7 => self.rst(interrupts, peripherals, 0x10),
      0xE7 => self.rst(interrupts, peripherals, 0x20),
      0xF7 => self.rst(interrupts, peripherals, 0x30),
      0xC8 => self.ret_cc(interrupts, peripherals, Cond::Z),
      0xD8 => self.ret_cc(interrupts, peripherals, Cond::C),
      0xE8 => self.add_sp_imm8(interrupts, peripherals),
      0xF8 => self.ld_hl_sp_imm8(interrupts, peripherals),
      0xC9 => self.ret(interrupts, peripherals),
      0xD9 => self.reti(interrupts, peripherals),
      0xE9 => self.jp_hl(interrupts, peripherals),
      0xF9 => self.ld_sp_hl(interrupts, peripherals),
      0xCA => self.jp_cc_imm16(interrupts, peripherals, Cond::Z),
      0xDA => self.jp_cc_imm16(interrupts, peripherals, Cond::C),
      0xEA => self.ld_direct_r8(interrupts, peripherals, Direct::D, Reg8::A),
      0xFA => self.ld_r8_direct(interrupts, peripherals, Reg8::A, Direct::D),
      0xCB => self.cb_prefix(interrupts, peripherals),
      0xDB => self.undefined(),
      0xEB => self.undefined(),
      0xFB => self.ei(interrupts, peripherals),
      0xCC => self.call_cc_imm16(interrupts, peripherals, Cond::Z),
      0xDC => self.call_cc_imm16(interrupts, peripherals, Cond::C),
      0xEC => self.undefined(),
      0xFC => self.undefined(),
      0xCD => self.call_imm16(interrupts, peripherals),
      0xDD => self.undefined(),
      0xED => self.undefined(),
      0xFD => self.undefined(),
      0xCE => self.adc_imm8(interrupts, peripherals),
      0xDE => self.sbc_imm8(interrupts, peripherals),
      0xEE => self.xor_imm8(interrupts, peripherals),
      0xFE => self.cp_imm8(interrupts, peripherals),
      0xCF => self.rst(interrupts, peripherals, 0x08),
      0xDF => self.rst(interrupts, peripherals, 0x18),
      0xEF => self.rst(interrupts, peripherals, 0x28),
      0xFF => self.rst(interrupts, peripherals, 0x38),
    }
  }
  // https://www.pastraiser.com/cpu/gameboy/gameboy_opcodes.html
  fn cb_decode_exec_fetch_cycle(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.opcode {
      0x00 => self.rlc_r8(interrupts, peripherals, Reg8::B),
      0x10 => self.rl_r8(interrupts, peripherals, Reg8::B),
      0x20 => self.sla_r8(interrupts, peripherals, Reg8::B),
      0x30 => self.swap_r8(interrupts, peripherals, Reg8::B),
      0x01 => self.rlc_r8(interrupts, peripherals, Reg8::C),
      0x11 => self.rl_r8(interrupts, peripherals, Reg8::C),
      0x21 => self.sla_r8(interrupts, peripherals, Reg8::C),
      0x31 => self.swap_r8(interrupts, peripherals, Reg8::C),
      0x02 => self.rlc_r8(interrupts, peripherals, Reg8::D),
      0x12 => self.rl_r8(interrupts, peripherals, Reg8::D),
      0x22 => self.sla_r8(interrupts, peripherals, Reg8::D),
      0x32 => self.swap_r8(interrupts, peripherals, Reg8::D),
      0x03 => self.rlc_r8(interrupts, peripherals, Reg8::E),
      0x13 => self.rl_r8(interrupts, peripherals, Reg8::E),
      0x23 => self.sla_r8(interrupts, peripherals, Reg8::E),
      0x33 => self.swap_r8(interrupts, peripherals, Reg8::E),
      0x04 => self.rlc_r8(interrupts, peripherals, Reg8::H),
      0x14 => self.rl_r8(interrupts, peripherals, Reg8::H),
      0x24 => self.sla_r8(interrupts, peripherals, Reg8::H),
      0x34 => self.swap_r8(interrupts, peripherals, Reg8::H),
      0x05 => self.rlc_r8(interrupts, peripherals, Reg8::L),
      0x15 => self.rl_r8(interrupts, peripherals, Reg8::L),
      0x25 => self.sla_r8(interrupts, peripherals, Reg8::L),
      0x35 => self.swap_r8(interrupts, peripherals, Reg8::L),
      0x06 => self.rlc_hl(interrupts, peripherals),
      0x16 => self.rl_hl(interrupts, peripherals),
      0x26 => self.sla_hl(interrupts, peripherals),
      0x36 => self.swap_hl(interrupts, peripherals),
      0x07 => self.rlc_r8(interrupts, peripherals, Reg8::A),
      0x17 => self.rl_r8(interrupts, peripherals, Reg8::A),
      0x27 => self.sla_r8(interrupts, peripherals, Reg8::A),
      0x37 => self.swap_r8(interrupts, peripherals, Reg8::A),
      0x08 => self.rrc_r8(interrupts, peripherals, Reg8::B),
      0x18 => self.rr_r8(interrupts, peripherals, Reg8::B),
      0x28 => self.sra_r8(interrupts, peripherals, Reg8::B),
      0x38 => self.srl_r8(interrupts, peripherals, Reg8::B),
      0x09 => self.rrc_r8(interrupts, peripherals, Reg8::C),
      0x19 => self.rr_r8(interrupts, peripherals, Reg8::C),
      0x29 => self.sra_r8(interrupts, peripherals, Reg8::C),
      0x39 => self.srl_r8(interrupts, peripherals, Reg8::C),
      0x0A => self.rrc_r8(interrupts, peripherals, Reg8::D),
      0x1A => self.rr_r8(interrupts, peripherals, Reg8::D),
      0x2A => self.sra_r8(interrupts, peripherals, Reg8::D),
      0x3A => self.srl_r8(interrupts, peripherals, Reg8::D),
      0x0B => self.rrc_r8(interrupts, peripherals, Reg8::E),
      0x1B => self.rr_r8(interrupts, peripherals, Reg8::E),
      0x2B => self.sra_r8(interrupts, peripherals, Reg8::E),
      0x3B => self.srl_r8(interrupts, peripherals, Reg8::E),
      0x0C => self.rrc_r8(interrupts, peripherals, Reg8::H),
      0x1C => self.rr_r8(interrupts, peripherals, Reg8::H),
      0x2C => self.sra_r8(interrupts, peripherals, Reg8::H),
      0x3C => self.srl_r8(interrupts, peripherals, Reg8::H),
      0x0D => self.rrc_r8(interrupts, peripherals, Reg8::L),
      0x1D => self.rr_r8(interrupts, peripherals, Reg8::L),
      0x2D => self.sra_r8(interrupts, peripherals, Reg8::L),
      0x3D => self.srl_r8(interrupts, peripherals, Reg8::L),
      0x0E => self.rrc_hl(interrupts, peripherals),
      0x1E => self.rr_hl(interrupts, peripherals),
      0x2E => self.sra_hl(interrupts, peripherals),
      0x3E => self.srl_hl(interrupts, peripherals),
      0x0F => self.rrc_r8(interrupts, peripherals, Reg8::A),
      0x1F => self.rr_r8(interrupts, peripherals, Reg8::A),
      0x2F => self.sra_r8(interrupts, peripherals, Reg8::A),
      0x3F => self.srl_r8(interrupts, peripherals, Reg8::A),
      0x40 => self.bit_r8(interrupts, peripherals, 0, Reg8::B),
      0x50 => self.bit_r8(interrupts, peripherals, 2, Reg8::B),
      0x60 => self.bit_r8(interrupts, peripherals, 4, Reg8::B),
      0x70 => self.bit_r8(interrupts, peripherals, 6, Reg8::B),
      0x41 => self.bit_r8(interrupts, peripherals, 0, Reg8::C),
      0x51 => self.bit_r8(interrupts, peripherals, 2, Reg8::C),
      0x61 => self.bit_r8(interrupts, peripherals, 4, Reg8::C),
      0x71 => self.bit_r8(interrupts, peripherals, 6, Reg8::C),
      0x42 => self.bit_r8(interrupts, peripherals, 0, Reg8::D),
      0x52 => self.bit_r8(interrupts, peripherals, 2, Reg8::D),
      0x62 => self.bit_r8(interrupts, peripherals, 4, Reg8::D),
      0x72 => self.bit_r8(interrupts, peripherals, 6, Reg8::D),
      0x43 => self.bit_r8(interrupts, peripherals, 0, Reg8::E),
      0x53 => self.bit_r8(interrupts, peripherals, 2, Reg8::E),
      0x63 => self.bit_r8(interrupts, peripherals, 4, Reg8::E),
      0x73 => self.bit_r8(interrupts, peripherals, 6, Reg8::E),
      0x44 => self.bit_r8(interrupts, peripherals, 0, Reg8::H),
      0x54 => self.bit_r8(interrupts, peripherals, 2, Reg8::H),
      0x64 => self.bit_r8(interrupts, peripherals, 4, Reg8::H),
      0x74 => self.bit_r8(interrupts, peripherals, 6, Reg8::H),
      0x45 => self.bit_r8(interrupts, peripherals, 0, Reg8::L),
      0x55 => self.bit_r8(interrupts, peripherals, 2, Reg8::L),
      0x65 => self.bit_r8(interrupts, peripherals, 4, Reg8::L),
      0x75 => self.bit_r8(interrupts, peripherals, 6, Reg8::L),
      0x46 => self.bit_hl(interrupts, peripherals, 0),
      0x56 => self.bit_hl(interrupts, peripherals, 2),
      0x66 => self.bit_hl(interrupts, peripherals, 4),
      0x76 => self.bit_hl(interrupts, peripherals, 6),
      0x47 => self.bit_r8(interrupts, peripherals, 0, Reg8::A),
      0x57 => self.bit_r8(interrupts, peripherals, 2, Reg8::A),
      0x67 => self.bit_r8(interrupts, peripherals, 4, Reg8::A),
      0x77 => self.bit_r8(interrupts, peripherals, 6, Reg8::A),
      0x48 => self.bit_r8(interrupts, peripherals, 1, Reg8::B),
      0x58 => self.bit_r8(interrupts, peripherals, 3, Reg8::B),
      0x68 => self.bit_r8(interrupts, peripherals, 5, Reg8::B),
      0x78 => self.bit_r8(interrupts, peripherals, 7, Reg8::B),
      0x49 => self.bit_r8(interrupts, peripherals, 1, Reg8::C),
      0x59 => self.bit_r8(interrupts, peripherals, 3, Reg8::C),
      0x69 => self.bit_r8(interrupts, peripherals, 5, Reg8::C),
      0x79 => self.bit_r8(interrupts, peripherals, 7, Reg8::C),
      0x4A => self.bit_r8(interrupts, peripherals, 1, Reg8::D),
      0x5A => self.bit_r8(interrupts, peripherals, 3, Reg8::D),
      0x6A => self.bit_r8(interrupts, peripherals, 5, Reg8::D),
      0x7A => self.bit_r8(interrupts, peripherals, 7, Reg8::D),
      0x4B => self.bit_r8(interrupts, peripherals, 1, Reg8::E),
      0x5B => self.bit_r8(interrupts, peripherals, 3, Reg8::E),
      0x6B => self.bit_r8(interrupts, peripherals, 5, Reg8::E),
      0x7B => self.bit_r8(interrupts, peripherals, 7, Reg8::E),
      0x4C => self.bit_r8(interrupts, peripherals, 1, Reg8::H),
      0x5C => self.bit_r8(interrupts, peripherals, 3, Reg8::H),
      0x6C => self.bit_r8(interrupts, peripherals, 5, Reg8::H),
      0x7C => self.bit_r8(interrupts, peripherals, 7, Reg8::H),
      0x4D => self.bit_r8(interrupts, peripherals, 1, Reg8::L),
      0x5D => self.bit_r8(interrupts, peripherals, 3, Reg8::L),
      0x6D => self.bit_r8(interrupts, peripherals, 5, Reg8::L),
      0x7D => self.bit_r8(interrupts, peripherals, 7, Reg8::L),
      0x4E => self.bit_hl(interrupts, peripherals, 1),
      0x5E => self.bit_hl(interrupts, peripherals, 3),
      0x6E => self.bit_hl(interrupts, peripherals, 5),
      0x7E => self.bit_hl(interrupts, peripherals, 7),
      0x4F => self.bit_r8(interrupts, peripherals, 1, Reg8::A),
      0x5F => self.bit_r8(interrupts, peripherals, 3, Reg8::A),
      0x6F => self.bit_r8(interrupts, peripherals, 5, Reg8::A),
      0x7F => self.bit_r8(interrupts, peripherals, 7, Reg8::A),
      0x80 => self.res_r8(interrupts, peripherals, 0, Reg8::B),
      0x90 => self.res_r8(interrupts, peripherals, 2, Reg8::B),
      0xA0 => self.res_r8(interrupts, peripherals, 4, Reg8::B),
      0xB0 => self.res_r8(interrupts, peripherals, 6, Reg8::B),
      0x81 => self.res_r8(interrupts, peripherals, 0, Reg8::C),
      0x91 => self.res_r8(interrupts, peripherals, 2, Reg8::C),
      0xA1 => self.res_r8(interrupts, peripherals, 4, Reg8::C),
      0xB1 => self.res_r8(interrupts, peripherals, 6, Reg8::C),
      0x82 => self.res_r8(interrupts, peripherals, 0, Reg8::D),
      0x92 => self.res_r8(interrupts, peripherals, 2, Reg8::D),
      0xA2 => self.res_r8(interrupts, peripherals, 4, Reg8::D),
      0xB2 => self.res_r8(interrupts, peripherals, 6, Reg8::D),
      0x83 => self.res_r8(interrupts, peripherals, 0, Reg8::E),
      0x93 => self.res_r8(interrupts, peripherals, 2, Reg8::E),
      0xA3 => self.res_r8(interrupts, peripherals, 4, Reg8::E),
      0xB3 => self.res_r8(interrupts, peripherals, 6, Reg8::E),
      0x84 => self.res_r8(interrupts, peripherals, 0, Reg8::H),
      0x94 => self.res_r8(interrupts, peripherals, 2, Reg8::H),
      0xA4 => self.res_r8(interrupts, peripherals, 4, Reg8::H),
      0xB4 => self.res_r8(interrupts, peripherals, 6, Reg8::H),
      0x85 => self.res_r8(interrupts, peripherals, 0, Reg8::L),
      0x95 => self.res_r8(interrupts, peripherals, 2, Reg8::L),
      0xA5 => self.res_r8(interrupts, peripherals, 4, Reg8::L),
      0xB5 => self.res_r8(interrupts, peripherals, 6, Reg8::L),
      0x86 => self.res_hl(interrupts, peripherals, 0),
      0x96 => self.res_hl(interrupts, peripherals, 2),
      0xA6 => self.res_hl(interrupts, peripherals, 4),
      0xB6 => self.res_hl(interrupts, peripherals, 6),
      0x87 => self.res_r8(interrupts, peripherals, 0, Reg8::A),
      0x97 => self.res_r8(interrupts, peripherals, 2, Reg8::A),
      0xA7 => self.res_r8(interrupts, peripherals, 4, Reg8::A),
      0xB7 => self.res_r8(interrupts, peripherals, 6, Reg8::A),
      0x88 => self.res_r8(interrupts, peripherals, 1, Reg8::B),
      0x98 => self.res_r8(interrupts, peripherals, 3, Reg8::B),
      0xA8 => self.res_r8(interrupts, peripherals, 5, Reg8::B),
      0xB8 => self.res_r8(interrupts, peripherals, 7, Reg8::B),
      0x89 => self.res_r8(interrupts, peripherals, 1, Reg8::C),
      0x99 => self.res_r8(interrupts, peripherals, 3, Reg8::C),
      0xA9 => self.res_r8(interrupts, peripherals, 5, Reg8::C),
      0xB9 => self.res_r8(interrupts, peripherals, 7, Reg8::C),
      0x8A => self.res_r8(interrupts, peripherals, 1, Reg8::D),
      0x9A => self.res_r8(interrupts, peripherals, 3, Reg8::D),
      0xAA => self.res_r8(interrupts, peripherals, 5, Reg8::D),
      0xBA => self.res_r8(interrupts, peripherals, 7, Reg8::D),
      0x8B => self.res_r8(interrupts, peripherals, 1, Reg8::E),
      0x9B => self.res_r8(interrupts, peripherals, 3, Reg8::E),
      0xAB => self.res_r8(interrupts, peripherals, 5, Reg8::E),
      0xBB => self.res_r8(interrupts, peripherals, 7, Reg8::E),
      0x8C => self.res_r8(interrupts, peripherals, 1, Reg8::H),
      0x9C => self.res_r8(interrupts, peripherals, 3, Reg8::H),
      0xAC => self.res_r8(interrupts, peripherals, 5, Reg8::H),
      0xBC => self.res_r8(interrupts, peripherals, 7, Reg8::H),
      0x8D => self.res_r8(interrupts, peripherals, 1, Reg8::L),
      0x9D => self.res_r8(interrupts, peripherals, 3, Reg8::L),
      0xAD => self.res_r8(interrupts, peripherals, 5, Reg8::L),
      0xBD => self.res_r8(interrupts, peripherals, 7, Reg8::L),
      0x8E => self.res_hl(interrupts, peripherals, 1),
      0x9E => self.res_hl(interrupts, peripherals, 3),
      0xAE => self.res_hl(interrupts, peripherals, 5),
      0xBE => self.res_hl(interrupts, peripherals, 7),
      0x8F => self.res_r8(interrupts, peripherals, 1, Reg8::A),
      0x9F => self.res_r8(interrupts, peripherals, 3, Reg8::A),
      0xAF => self.res_r8(interrupts, peripherals, 5, Reg8::A),
      0xBF => self.res_r8(interrupts, peripherals, 7, Reg8::A),
      0xC0 => self.set_r8(interrupts, peripherals, 0, Reg8::B),
      0xD0 => self.set_r8(interrupts, peripherals, 2, Reg8::B),
      0xE0 => self.set_r8(interrupts, peripherals, 4, Reg8::B),
      0xF0 => self.set_r8(interrupts, peripherals, 6, Reg8::B),
      0xC1 => self.set_r8(interrupts, peripherals, 0, Reg8::C),
      0xD1 => self.set_r8(interrupts, peripherals, 2, Reg8::C),
      0xE1 => self.set_r8(interrupts, peripherals, 4, Reg8::C),
      0xF1 => self.set_r8(interrupts, peripherals, 6, Reg8::C),
      0xC2 => self.set_r8(interrupts, peripherals, 0, Reg8::D),
      0xD2 => self.set_r8(interrupts, peripherals, 2, Reg8::D),
      0xE2 => self.set_r8(interrupts, peripherals, 4, Reg8::D),
      0xF2 => self.set_r8(interrupts, peripherals, 6, Reg8::D),
      0xC3 => self.set_r8(interrupts, peripherals, 0, Reg8::E),
      0xD3 => self.set_r8(interrupts, peripherals, 2, Reg8::E),
      0xE3 => self.set_r8(interrupts, peripherals, 4, Reg8::E),
      0xF3 => self.set_r8(interrupts, peripherals, 6, Reg8::E),
      0xC4 => self.set_r8(interrupts, peripherals, 0, Reg8::H),
      0xD4 => self.set_r8(interrupts, peripherals, 2, Reg8::H),
      0xE4 => self.set_r8(interrupts, peripherals, 4, Reg8::H),
      0xF4 => self.set_r8(interrupts, peripherals, 6, Reg8::H),
      0xC5 => self.set_r8(interrupts, peripherals, 0, Reg8::L),
      0xD5 => self.set_r8(interrupts, peripherals, 2, Reg8::L),
      0xE5 => self.set_r8(interrupts, peripherals, 4, Reg8::L),
      0xF5 => self.set_r8(interrupts, peripherals, 6, Reg8::L),
      0xC6 => self.set_hl(interrupts, peripherals, 0),
      0xD6 => self.set_hl(interrupts, peripherals, 2),
      0xE6 => self.set_hl(interrupts, peripherals, 4),
      0xF6 => self.set_hl(interrupts, peripherals, 6),
      0xC7 => self.set_r8(interrupts, peripherals, 0, Reg8::A),
      0xD7 => self.set_r8(interrupts, peripherals, 2, Reg8::A),
      0xE7 => self.set_r8(interrupts, peripherals, 4, Reg8::A),
      0xF7 => self.set_r8(interrupts, peripherals, 6, Reg8::A),
      0xC8 => self.set_r8(interrupts, peripherals, 1, Reg8::B),
      0xD8 => self.set_r8(interrupts, peripherals, 3, Reg8::B),
      0xE8 => self.set_r8(interrupts, peripherals, 5, Reg8::B),
      0xF8 => self.set_r8(interrupts, peripherals, 7, Reg8::B),
      0xC9 => self.set_r8(interrupts, peripherals, 1, Reg8::C),
      0xD9 => self.set_r8(interrupts, peripherals, 3, Reg8::C),
      0xE9 => self.set_r8(interrupts, peripherals, 5, Reg8::C),
      0xF9 => self.set_r8(interrupts, peripherals, 7, Reg8::C),
      0xCA => self.set_r8(interrupts, peripherals, 1, Reg8::D),
      0xDA => self.set_r8(interrupts, peripherals, 3, Reg8::D),
      0xEA => self.set_r8(interrupts, peripherals, 5, Reg8::D),
      0xFA => self.set_r8(interrupts, peripherals, 7, Reg8::D),
      0xCB => self.set_r8(interrupts, peripherals, 1, Reg8::E),
      0xDB => self.set_r8(interrupts, peripherals, 3, Reg8::E),
      0xEB => self.set_r8(interrupts, peripherals, 5, Reg8::E),
      0xFB => self.set_r8(interrupts, peripherals, 7, Reg8::E),
      0xCC => self.set_r8(interrupts, peripherals, 1, Reg8::H),
      0xDC => self.set_r8(interrupts, peripherals, 3, Reg8::H),
      0xEC => self.set_r8(interrupts, peripherals, 5, Reg8::H),
      0xFC => self.set_r8(interrupts, peripherals, 7, Reg8::H),
      0xCD => self.set_r8(interrupts, peripherals, 1, Reg8::L),
      0xDD => self.set_r8(interrupts, peripherals, 3, Reg8::L),
      0xED => self.set_r8(interrupts, peripherals, 5, Reg8::L),
      0xFD => self.set_r8(interrupts, peripherals, 7, Reg8::L),
      0xCE => self.set_hl(interrupts, peripherals, 1),
      0xDE => self.set_hl(interrupts, peripherals, 3),
      0xEE => self.set_hl(interrupts, peripherals, 5),
      0xFE => self.set_hl(interrupts, peripherals, 7),
      0xCF => self.set_r8(interrupts, peripherals, 1, Reg8::A),
      0xDF => self.set_r8(interrupts, peripherals, 3, Reg8::A),
      0xEF => self.set_r8(interrupts, peripherals, 5, Reg8::A),
      0xFF => self.set_r8(interrupts, peripherals, 7, Reg8::A),
    }
  }
  fn interrupt_dispatch(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.ime = false;
        self.command_cycle += 1;
      },
      1 => {
        self.val16 = self.regs.pc;
        self.command_cycle += 1;
      },
      2 => {
        let [lo, hi] = u16::to_le_bytes(self.val16);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, hi);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      3 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, self.val8);
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
        self.command_cycle += 1;
      },
      4 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  // 8-bit operations
  fn ld_r8_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.write_r8(dst, self.read_r8(src));
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_r8_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.write_r8(dst, val);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_r8_indirect(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8, src: Indirect) {
    match self.command_cycle {
      0 => {
        let val = self.read_indirect(interrupts, peripherals, src);
        self.write_r8(dst, val);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_indirect_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Indirect, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.write_indirect(interrupts, peripherals, dst, self.read_r8(src));
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_indirect_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Indirect) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        self.write_indirect(interrupts, peripherals, dst, self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_r8_direct(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8, src: Direct) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
        if let Direct::DFF = src {
          self.val16 = 0xff00 | (self.val8 as u16);
          self.command_cycle += 1;
        }
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.write_r8(dst, peripherals.read(interrupts, self.val16));
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_direct_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Direct, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
        if let Direct::DFF = dst {
          self.val16 = 0xff00 | (self.val8 as u16);
          self.command_cycle += 1;
        }
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        peripherals.write(interrupts, self.val16, self.read_r8(src));
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn add_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let (result, carry) = self.regs.a.overflowing_add(val);
        let half_carry = (self.regs.a & 0x0f).checked_add(val | 0xf0).is_none();
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(half_carry);
        self.regs.set_cf(carry);
        self.regs.a = result;
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn add_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        let (result, carry) = self.regs.a.overflowing_add(val);
        let half_carry = (self.regs.a & 0x0f).checked_add(val | 0xf0).is_none();
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(half_carry);
        self.regs.set_cf(carry);
        self.regs.a = result;
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn add_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        let (result, carry) = self.regs.a.overflowing_add(val);
        let half_carry = (self.regs.a & 0x0f).checked_add(val | 0xf0).is_none();
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(half_carry);
        self.regs.set_cf(carry);
        self.regs.a = result;
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn adc_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let cy = self.regs.cf() as u8;
        let result = self.regs.a.wrapping_add(val).wrapping_add(cy);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(
          (self.regs.a & 0xf) + (val & 0xf) + cy > 0xf
        );
        self.regs.set_cf(
          self.regs.a as u16 + val as u16 + cy as u16 > 0xff
        );
        self.regs.a = result;
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn adc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        let cy = self.regs.cf() as u8;
        let result = self.regs.a.wrapping_add(val).wrapping_add(cy);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(
          (self.regs.a & 0xf) + (val & 0xf) + cy > 0xf
        );
        self.regs.set_cf(
          self.regs.a as u16 + val as u16 + cy as u16 > 0xff
        );
        self.regs.a = result;
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn adc_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        let cy = self.regs.cf() as u8;
        let result = self.regs.a.wrapping_add(val).wrapping_add(cy);
        self.regs.set_zf(result == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(
          (self.regs.a & 0xf) + (val & 0xf) + cy > 0xf
        );
        self.regs.set_cf(
          self.regs.a as u16 + val as u16 + cy as u16 > 0xff
        );
        self.regs.a = result;
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sub_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a = self.alu_sub(val, false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sub_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.regs.a = self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sub_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.regs.a = self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sbc_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a = self.alu_sub(val, self.regs.cf());
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sbc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.regs.a = self.alu_sub(val, self.regs.cf());
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sbc_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.regs.a = self.alu_sub(val, self.regs.cf());
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn cp_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.alu_sub(val, false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn cp_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn cp_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn and_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a &= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn and_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.regs.a &= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn and_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.regs.a &= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn or_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a |= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn or_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.regs.a |= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn or_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.regs.a |= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn xor_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a ^= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn xor_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.regs.a ^= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn xor_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals);
        self.regs.a ^= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn inc_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let new_val = val.wrapping_add(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(val & 0xf == 0xf);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn inc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.val8 = val.wrapping_add(1);
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(val & 0xf == 0xf);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn dec_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let new_val = val.wrapping_sub(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(val & 0xf == 0);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn dec_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.val8 = val.wrapping_sub(1);
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(val & 0xf == 0);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rlca(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.a = self.alu_rlc(self.regs.a);
        self.regs.set_zf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rla(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.a = self.alu_rl(self.regs.a);
        self.regs.set_zf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rrca(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.a = self.alu_rrc(self.regs.a);
        self.regs.set_zf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rra(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.a = self.alu_rr(self.regs.a);
        self.regs.set_zf(false);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rlc_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.alu_rlc(self.read_r8(src));
        self.write_r8(src, val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rlc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.alu_rlc(peripherals.read(interrupts, self.regs.hl()));
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rl_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.alu_rl(self.read_r8(src));
        self.write_r8(src, val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rl_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.alu_rl(peripherals.read(interrupts, self.regs.hl()));
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rrc_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.alu_rrc(self.read_r8(src));
        self.write_r8(src, val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rrc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.alu_rrc(peripherals.read(interrupts, self.regs.hl()));
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rr_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.alu_rr(self.read_r8(src));
        self.write_r8(src, val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rr_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.alu_rr(peripherals.read(interrupts, self.regs.hl()));
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sla_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let co = val & 0x80;
        let new_val = val << 1;
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co > 0);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sla_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        let co = val & 0x80;
        self.val8 = val << 1;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co > 0);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sra_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let co = val & 0x01;
        let hi = val & 0x80;
        let new_val = (val >> 1) | hi;
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co > 0);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn sra_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        let co = val & 0x01;
        let hi = val & 0x80;
        self.val8 = (val >> 1) | hi;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co != 0);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn srl_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let co = val & 0x01;
        let new_val = val >> 1;
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co != 0);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn srl_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        let co = val & 0x01;
        self.val8 = val >> 1;
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(co != 0);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn swap_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let new_val = (val >> 4) | (val << 4);
        self.regs.set_zf(val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn swap_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl());
        self.val8 = (val >> 4) | (val << 4);
        self.regs.set_zf(val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn bit_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, bit: usize, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src) & (1 << bit);
        self.regs.set_zf(val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn bit_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, bit: usize) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(interrupts, self.regs.hl()) & (1 << bit);
        self.regs.set_zf(val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn set_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, bit: usize, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src) | (1 << bit);
        self.write_r8(src, val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn set_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, bit: usize) {
    match self.command_cycle {
      0 => {
        self.val8 = peripherals.read(interrupts, self.regs.hl()) | (1 << bit);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn res_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, bit: usize, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src) & !(1 << bit);
        self.write_r8(src, val);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn res_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, bit: usize) {
    match self.command_cycle {
      0 => {
        self.val8 = peripherals.read(interrupts, self.regs.hl()) & !(1 << bit);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(interrupts, self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn jp_imm16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn jp_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.pc = self.regs.hl();
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn jr_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        self.regs.pc = self.regs.pc.wrapping_add((self.val8 as i8) as u16);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn call_imm16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.command_cycle += 1;
      },
      3 => {
        let [lo, hi] = u16::to_le_bytes(self.regs.pc);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, hi);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      4 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, self.val8);
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      5 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ret(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.command_cycle += 1;
      },
      1 => {
        let hi = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn reti(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.ime = true;
        self.val8 = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.command_cycle += 1;
      },
      1 => {
        let hi = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn jp_cc_imm16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        if self.check_cond(cond) {
          self.regs.pc = self.val16;
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals);
        }
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn jr_cc_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        if self.check_cond(cond) {
          self.regs.pc = self.regs.pc.wrapping_add((self.val8 as i8) as u16);
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals);
        }
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn call_cc_imm16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        if self.check_cond(cond) {
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals);
        }
      },
      3 => {
        let [lo, hi] = u16::to_le_bytes(self.regs.pc);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, hi);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      4 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, self.val8);
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      5 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ret_cc(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.command_cycle += 1;
      },
      1 => {
        if self.check_cond(cond) {
          self.val8 = peripherals.read(interrupts, self.regs.sp);
          self.regs.sp = self.regs.sp.wrapping_add(1);
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals);
        }
      },
      2 => {
        let hi = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      3 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      4 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn rst(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, addr: u8) {
    match self.command_cycle {
      0 => {
        self.command_cycle += 1;
      },
      1 => {
        let [lo, hi] = u16::to_le_bytes(self.regs.pc);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, hi);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      2 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, self.val8);
        self.regs.pc = addr as u16;
        self.command_cycle += 1;
      }
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn halt(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.command_cycle += 1;
      },
      1 => {
        if interrupts.get_interrupt() > 0 {
          self.command_cycle = 0;
          if self.ime {
            self.state = State::InterruptDispatch;
          } else {
            // This causes halt bug. (https://gbdev.io/pandocs/halt.html#halt-bug)
            self.opcode = peripherals.read(interrupts, self.regs.pc);
            // self.prefetch(interrupts, peripherals);
            // self.decode_exec_fetch_cycle(interrupts, peripherals);
          }
        } else {
          self.command_cycle += 1;
        }
      },
      2 => {
        self.state = State::Halt;
        self.command_cycle = 0;
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn stop(&mut self) {
    panic!("STOP");
  }
  fn di(&mut self, interrupts: &interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.ime = false;
        self.opcode = self.read_imm8(interrupts, peripherals);
        self.state = State::Running;
        self.command_cycle = 0;
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ei(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.prefetch(interrupts, peripherals);
        self.ime = true;
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ccf(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(!self.regs.cf());
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn scf(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(true);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn nop(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn daa(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
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
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn cpl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.a = !self.regs.a;
        self.regs.set_nf(true);
        self.regs.set_hf(true);
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  // 16-bit operations
  fn ld_r16_imm16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg16) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.write_r16(dst, u16::from_le_bytes([self.val8, hi]));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_direct_sp(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(interrupts, peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let hi = self.read_imm8(interrupts, peripherals);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        peripherals.write(interrupts, self.val16, self.regs.sp as u8);
        self.command_cycle += 1;
      },
      3 => {
        peripherals.write(interrupts, self.val16.wrapping_add(1), (self.regs.sp >> 8) as u8);
        self.command_cycle += 1;
      },
      4 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_sp_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.regs.sp = self.regs.hl();
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn ld_hl_sp_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val16 = self.read_imm8(interrupts, peripherals) as i8 as u16;
        self.command_cycle += 1;
      },
      1 => {
        let val = self.regs.sp.wrapping_add(self.val16);
        self.write_r16(Reg16::HL, val);
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf(check_add_carry(3, self.regs.sp, self.val16));
        self.regs.set_cf(check_add_carry(7, self.regs.sp, self.val16));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn push_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg16) {
    match self.command_cycle {
      0 => {
        self.val16 = self.read_r16(src);
        self.command_cycle += 1;
      },
      1 => {
        let [lo, hi] = u16::to_le_bytes(self.val16);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, hi);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      2 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(interrupts, self.regs.sp, self.val8);
        self.command_cycle += 1;
      }
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn pop_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg16) {
    match self.command_cycle {
      0 => {
        self.val8 = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.command_cycle += 1;
      },
      1 => {
        let hi = peripherals.read(interrupts, self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.write_r16(dst, u16::from_le_bytes([self.val8, hi]));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn add_hl_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg16) {
    match self.command_cycle {
      0 => {
        let hl = self.regs.hl();
        let val = self.read_r16(src);
        self.regs.set_nf(false);
        self.regs.set_hf(check_add_carry(11, hl, val));
        self.regs.set_cf(hl > 0xffff - val);
        self.write_r16(Reg16::HL, hl.wrapping_add(val));
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn add_sp_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(interrupts, peripherals) as i8 as i16 as u16;
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf(check_add_carry(3, self.regs.sp, val));
        self.regs.set_cf(check_add_carry(7, self.regs.sp, val));
        self.regs.sp = self.regs.sp.wrapping_add(val);
        self.command_cycle += 1;
      },
      1 => {
        self.command_cycle += 1;
      },
      2 => {
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn inc_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg16) {
    match self.command_cycle {
      0 => {
        self.write_r16(src, self.read_r16(src).wrapping_add(1));
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn dec_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg16) {
    match self.command_cycle {
      0 => {
        self.write_r16(src, self.read_r16(src).wrapping_sub(1));
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
  fn undefined(&mut self) {
    panic!("Undefined opcode {:02x}", self.opcode);
  }
  fn cb_prefix(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.command_cycle += 1;
      },
      1 => {
        self.opcode = self.read_imm8(interrupts, peripherals);
        self.command_cycle = 0;
        self.cb = true;
        self.cb_decode_exec_fetch_cycle(interrupts, peripherals);
      },
      _ => panic!("Unexpected error."),
    }
  }
}
