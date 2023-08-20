use crate::register;
use crate::interrupts;
use crate::peripherals;

/// Tests if addition results in a carry from the specified bit.
/// Does not support overflow, so cannot be used to check carry from the leftmost bit
#[inline(always)]
fn test_add_carry_bit(bit: usize, a: u16, b: u16) -> bool {
  // Create a mask that includes the specified bit and 1-bits on the right side
  // e.g. for u8:
  //   bit=0 -> 0000 0001
  //   bit=3 -> 0000 1111
  //   bit=6 -> 0111 1111
  let x = 1u16 << bit;
  let mask = x | x.wrapping_sub(1);
  (a & mask) + (b & mask) > mask
}

/// Isolates the rightmost 1-bit leaving all other bits as 0
/// e.g. 1010 1000 -> 0000 1000
///
/// Equivalent to Intel BMI1 instruction BLSI
#[inline(always)]
fn isolate_rightmost_one(x: u8) -> u8 {
  // Unsigned negation: -x == !x + 1
  let minus_x = (!x).wrapping_add(1);
  // Hacker's Delight 2nd ed, 2-1 Manipulating Rightmost Bits
  x & minus_x
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
      state: State::Running,
      regs: register::Registers::new(),
      ime: false,
      opcode: 0x00,
      command_cycle: 0,
      val8: 0,
      val16: 0,
    }
  }

  fn prefetch(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, addr: u16) {
    self.opcode = peripherals.read(addr);
    let interrupt = interrupts.get_interrupt();
    if self.ime && interrupt != 0 {
      self.state = State::InterruptDispatch;
    } else {
      self.regs.pc = addr.wrapping_add(1);
      self.state = State::Running;
    }
    self.command_cycle = 0;
  }

  // read absolute addr specified by pc register
  fn read_imm8(&mut self, peripherals: &mut peripherals::Peripherals) -> u8 {
    let ret = peripherals.read(self.regs.pc);
    self.regs.pc = self.regs.pc.wrapping_add(1);
    ret
  }

  fn read_imm16(&mut self, peripherals: &mut peripherals::Peripherals) -> u16 {
    let lo = self.read_imm8(peripherals);
    let hi = self.read_imm8(peripherals);
    u16::from_le_bytes([lo, hi])
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
    self.regs.set_cf(co != 0);
    new_val
  }
  fn alu_rlc(&mut self, val: u8) -> u8 {
    let co = val & 0x80;
    let new_val = val.rotate_left(1);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co != 0);
    new_val
  }
  fn alu_rr(&mut self, val: u8) -> u8 {
    let ci = self.regs.cf() as u8;
    let co = val & 0x01;
    let new_val = (val >> 1) | (ci << 7);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co != 0);
    new_val
  }
  fn alu_rrc(&mut self, val: u8) -> u8 {
    let co = val & 0x01;
    let new_val = val.rotate_right(1);
    self.regs.set_zf(new_val == 0);
    self.regs.set_nf(false);
    self.regs.set_hf(false);
    self.regs.set_cf(co != 0);
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
  fn write_r8(&mut self, dst: Reg8, data: u8) {
    match dst {
      Reg8::A => self.regs.a = data,
      Reg8::B => self.regs.b = data,
      Reg8::C => self.regs.c = data,
      Reg8::D => self.regs.d = data,
      Reg8::E => self.regs.e = data,
      Reg8::H => self.regs.h = data,
      Reg8::L => self.regs.l = data,
    }
  }

  // read data from 16bit register
  fn read_r16(&self, src: Reg16) -> u16 {
    match src {
      Reg16::BC => self.regs.bc(),
      Reg16::DE => self.regs.de(),
      Reg16::HL => self.regs.hl(),
      _ => panic!("Unexpected error."),
    }
  }

  // write data to 16bit register
  fn write_r16(&mut self, dst: Reg16, data: u16) {
    match dst {
      Reg16::BC => {
        self.regs.b = (data >> 8) as u8;
        self.regs.c = data as u8;
      },
      Reg16::DE => {
        self.regs.d = (data >> 8) as u8;
        self.regs.e = data as u8;
      },
      Reg16::HL => {
        self.regs.h = (data >> 8) as u8;
        self.regs.l = data as u8;
      },
      _ => panic!("Unexpected error."),
    }
  }

  // read absolute addr specified by 16bit reg
  fn read_indirect(&mut self, peripherals: &mut peripherals::Peripherals, src: Indirect) -> u8 {
    match src {
      Indirect::BC => peripherals.read(self.regs.bc()),
      Indirect::DE => peripherals.read(self.regs.de()),
      Indirect::HL => peripherals.read(self.regs.hl()),
      Indirect::CFF => peripherals.read(0xff00 | (self.regs.c as u16)),
      Indirect::HLD => {
        let addr = self.regs.hl();
        self.write_r16(Reg16::HL, addr.wrapping_sub(1));
        peripherals.read(addr)
      },
      Indirect::HLI => {
        let addr = self.regs.hl();
        self.write_r16(Reg16::HL, addr.wrapping_add(1));
        peripherals.read(addr)
      },
    }
  }

  // write data to absolute addr specified by 16bit reg
  fn write_indirect(&mut self, peripherals: &mut peripherals::Peripherals, dst: Indirect, data: u8) {
    match dst {
      Indirect::BC => peripherals.write(self.regs.bc(), data),
      Indirect::DE => peripherals.write(self.regs.de(), data),
      Indirect::HL => peripherals.write(self.regs.hl(), data),
      Indirect::CFF => peripherals.write(0xff00 | (self.regs.c as u16), data),
      Indirect::HLD => {
        let addr = self.regs.hl();
        peripherals.write(addr, data);
        self.write_r16(Reg16::HL, addr.wrapping_sub(1));
      },
      Indirect::HLI => {
        let addr = self.regs.hl();
        peripherals.write(addr, data);
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
          self.prefetch(interrupts, peripherals, self.regs.pc);
        }
      }
    }
  }

  fn decode_exec_fetch_cycle(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.opcode {
      0x7f => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::A),
      0x78 => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::B),
      0x79 => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::C),
      0x7a => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::D),
      0x7b => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::E),
      0x7c => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::H),
      0x7d => self.ld_r8_r8(interrupts, peripherals, Reg8::A, Reg8::L),
      0x47 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::A),
      0x40 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::B),
      0x41 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::C),
      0x42 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::D),
      0x43 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::E),
      0x44 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::H),
      0x45 => self.ld_r8_r8(interrupts, peripherals, Reg8::B, Reg8::L),
      0x4f => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::A),
      0x48 => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::B),
      0x49 => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::C),
      0x4a => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::D),
      0x4b => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::E),
      0x4c => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::H),
      0x4d => self.ld_r8_r8(interrupts, peripherals, Reg8::C, Reg8::L),
      0x57 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::A),
      0x50 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::B),
      0x51 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::C),
      0x52 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::D),
      0x53 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::E),
      0x54 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::H),
      0x55 => self.ld_r8_r8(interrupts, peripherals, Reg8::D, Reg8::L),
      0x5f => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::A),
      0x58 => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::B),
      0x59 => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::C),
      0x5a => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::D),
      0x5b => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::E),
      0x5c => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::H),
      0x5d => self.ld_r8_r8(interrupts, peripherals, Reg8::E, Reg8::L),
      0x67 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::A),
      0x60 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::B),
      0x61 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::C),
      0x62 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::D),
      0x63 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::E),
      0x64 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::H),
      0x65 => self.ld_r8_r8(interrupts, peripherals, Reg8::H, Reg8::L),
      0x6f => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::A),
      0x68 => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::B),
      0x69 => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::C),
      0x6a => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::D),
      0x6b => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::E),
      0x6c => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::H),
      0x6d => self.ld_r8_r8(interrupts, peripherals, Reg8::L, Reg8::L),
      0x3e => self.ld_r8_imm8(interrupts, peripherals, Reg8::A),
      0x06 => self.ld_r8_imm8(interrupts, peripherals, Reg8::B),
      0x0e => self.ld_r8_imm8(interrupts, peripherals, Reg8::C),
      0x16 => self.ld_r8_imm8(interrupts, peripherals, Reg8::D),
      0x1e => self.ld_r8_imm8(interrupts, peripherals, Reg8::E),
      0x26 => self.ld_r8_imm8(interrupts, peripherals, Reg8::H),
      0x2e => self.ld_r8_imm8(interrupts, peripherals, Reg8::L),
      0x7e => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::HL),
      0x46 => self.ld_r8_indirect(interrupts, peripherals, Reg8::B, Indirect::HL),
      0x4e => self.ld_r8_indirect(interrupts, peripherals, Reg8::C, Indirect::HL),
      0x56 => self.ld_r8_indirect(interrupts, peripherals, Reg8::D, Indirect::HL),
      0x5e => self.ld_r8_indirect(interrupts, peripherals, Reg8::E, Indirect::HL),
      0x66 => self.ld_r8_indirect(interrupts, peripherals, Reg8::H, Indirect::HL),
      0x6e => self.ld_r8_indirect(interrupts, peripherals, Reg8::L, Indirect::HL),
      0x77 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::A),
      0x70 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::B),
      0x71 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::C),
      0x72 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::D),
      0x73 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::E),
      0x74 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::H),
      0x75 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HL, Reg8::L),
      0x36 => self.ld_indirect_imm8(interrupts, peripherals, Indirect::HL),
      0x0a => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::BC),
      0x1a => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::DE),
      0x02 => self.ld_indirect_r8(interrupts, peripherals, Indirect::BC, Reg8::A),
      0x12 => self.ld_indirect_r8(interrupts, peripherals, Indirect::DE, Reg8::A),
      0xfa => self.ld_r8_direct(interrupts, peripherals, Reg8::A, Direct::D),
      0xea => self.ld_direct_r8(interrupts, peripherals, Direct::D, Reg8::A),
      0xf2 => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::CFF),
      0xe2 => self.ld_indirect_r8(interrupts, peripherals, Indirect::CFF, Reg8::A),
      0xf0 => self.ld_r8_direct(interrupts, peripherals, Reg8::A, Direct::DFF),
      0xe0 => self.ld_direct_r8(interrupts, peripherals, Direct::DFF, Reg8::A),
      0x3a => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::HLD),
      0x32 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HLD, Reg8::A),
      0x2a => self.ld_r8_indirect(interrupts, peripherals, Reg8::A, Indirect::HLI),
      0x22 => self.ld_indirect_r8(interrupts, peripherals, Indirect::HLI, Reg8::A),

      0x00 => self.nop(interrupts, peripherals),
      _ => panic!("Undefined opcode {}", self.opcode),
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
        peripherals.write(self.regs.sp, hi);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      3 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        peripherals.write(self.regs.sp, self.val8);
        let interrupt: u8 = isolate_rightmost_one(interrupts.get_interrupt()); // get highest priority interrupt
        interrupts.ack_interrupt(interrupt);
        self.regs.pc = match interrupt {
          interrupts::VBLANK => 0x0040,
          interrupts::STAT => 0x0048,
          interrupts::TIMER => 0x0050,
          interrupts::SERIAL => 0x0058,
          interrupts::JOYPAD => 0x0060,
          _ => 0x0000,
        };
        self.command_cycle += 1;
      },
      4 => {
        if self.ime {
          panic!("expect ime false.");
        }
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r8_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.write_r8(dst, self.read_r8(src));
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r8_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8) {
    match self.command_cycle {
      0 => {
        let data = self.read_imm8(peripherals);
        self.write_r8(dst, data);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r8_indirect(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8, src: Indirect) {
    match self.command_cycle {
      0 => {
        let data = self.read_indirect(peripherals, src);
        self.write_r8(dst, data);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_indirect_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Indirect, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.write_indirect(peripherals, dst, self.read_r8(src));
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_indirect_imm8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Indirect) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        self.write_indirect(peripherals, dst, self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r8_direct(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg8, src: Direct) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
        if let Direct::DFF = src {
          self.val16 = 0xff00 | (self.val8 as u16);
          self.command_cycle += 1;
        }
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.write_r8(dst, peripherals.read(self.val16));
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_direct_r8(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Direct, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
        if let Direct::DFF = dst {
          self.val16 = 0xff00 | (self.val8 as u16);
          self.command_cycle += 1;
        }
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        peripherals.write(self.val16, self.read_r8(src));
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r16_imm16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg16) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.write_r16(dst, u16::from_le_bytes([lo, hi]));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_imm16_sp(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        peripherals.write(self.val16, self.regs.sp as u8);
        self.command_cycle += 1;
      },
      3 => {
        peripherals.write(self.val16.wrapping_add(1), (self.regs.sp >> 8) as u8);
        self.command_cycle += 1;
      },
      4 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_hl_sp_e(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val16 = self.regs.hl() as i8 as u16;
        self.command_cycle += 1;
      },
      1 => {
        let data = self.regs.sp.wrapping_add(self.val16);
        self.write_r16(Reg16::HL, data);
        self.regs.set_zf(false);
        self.regs.set_nf(false);
        self.regs.set_hf(test_add_carry_bit(3, self.regs.sp, self.val16));
        self.regs.set_cf(test_add_carry_bit(7, self.regs.sp, self.val16));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn push_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg16) {
    match self.command_cycle {
      0 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.command_cycle += 1;
      },
      1 => {
        let [lo, hi] = u16::to_le_bytes(self.read_r16(src));
        peripherals.write(self.regs.sp, hi);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      2 => {
        peripherals.write(self.regs.sp, self.val8);
        self.command_cycle += 1;
      }
      3 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn pop_r16(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, dst: Reg16) {
    match self.command_cycle {
      0 => {
        self.val8 = peripherals.read(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.command_cycle += 1;
      },
      1 => {
        let hi = peripherals.read(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.write_r16(dst, u16::from_le_bytes([self.val8, hi]));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn add_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn add_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn add_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn adc_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn adc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn adc_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn sub_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a = self.alu_sub(val, false);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn sub_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.regs.a = self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn sub_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
        self.regs.a = self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn sbc_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a = self.alu_sub(val, self.regs.cf());
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn sbc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.regs.a = self.alu_sub(val, self.regs.cf());
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn sbc_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
        self.regs.a = self.alu_sub(val, self.regs.cf());
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn cp_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.alu_sub(val, false);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn cp_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn cp_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
        self.alu_sub(val, false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn inc_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let new_val = val.wrapping_add(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(val & 0xf == 0xf);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn inc_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.val8 = val.wrapping_add(1);
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(val & 0xf == 0xf);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn dec_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        let new_val = val.wrapping_sub(1);
        self.regs.set_zf(new_val == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(val & 0xf == 0);
        self.write_r8(src, new_val);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn dec_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.val8 = val.wrapping_sub(1);
        self.regs.set_zf(self.val8 == 0);
        self.regs.set_nf(true);
        self.regs.set_hf(val & 0xf == 0);
        self.command_cycle += 1;
      },
      1 => {
        peripherals.write(self.regs.hl(), self.val8);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn and_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a &= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn and_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.regs.a &= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn and_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
        self.regs.a &= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(true);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn or_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a |= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn or_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.regs.a |= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn or_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
        self.regs.a |= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn xor_r(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, src: Reg8) {
    match self.command_cycle {
      0 => {
        let val = self.read_r8(src);
        self.regs.a ^= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn xor_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = peripherals.read(self.regs.hl());
        self.regs.a ^= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn xor_n(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        let val = self.read_imm8(peripherals);
        self.regs.a ^= val;
        self.regs.set_zf(self.regs.a == 0);
        self.regs.set_nf(false);
        self.regs.set_hf(false);
        self.regs.set_cf(false);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
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
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn jp_nn(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn jp_hl(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        // self.regs.pc = self.regs.hl();
        // self.prefetch(interrupts, peripherals, );
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn jp_cc_nn(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        if self.check_cond(cond) {
          self.regs.pc = self.val16;
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals, self.regs.pc);
        }
      },
      3 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn jr_e(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        self.regs.pc = self.regs.pc.wrapping_add(self.val8 as i8 as u16);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn jr_cc_e(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        if self.check_cond(cond) {
          self.regs.pc = self.regs.pc.wrapping_add(self.val8 as i8 as u16);
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals, self.regs.pc);
        }
      },
      2 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn call(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.command_cycle += 1;
      },
      3 => {
        let [lo, hi] = u16::to_le_bytes(self.val16);
        peripherals.write(self.regs.sp, hi);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      4 => {
        peripherals.write(self.regs.sp, self.val8);
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      5 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn call_cc(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals, cond: Cond) {
    match self.command_cycle {
      0 => {
        self.val8 = self.read_imm8(peripherals);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.val8;
        let hi = self.read_imm8(peripherals);
        self.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        if self.check_cond(cond) {
          self.regs.sp = self.regs.sp.wrapping_sub(1);
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals, self.regs.pc);
        }
      },
      3 => {
        let [lo, hi] = u16::to_le_bytes(self.val16);
        peripherals.write(self.regs.sp, hi);
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        self.val8 = lo;
        self.command_cycle += 1;
      },
      4 => {
        peripherals.write(self.regs.sp, self.val8);
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      5 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ret(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.val8 = peripherals.read(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.command_cycle += 1;
      },
      1 => {
        let hi = peripherals.read(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
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
          self.ime = true;
          self.val8 = peripherals.read(self.regs.sp);
          self.regs.sp = self.regs.sp.wrapping_add(1);
          self.command_cycle += 1;
        } else {
          self.prefetch(interrupts, peripherals, self.regs.pc);
        }
      },
      2 => {
        let hi = peripherals.read(self.regs.sp);
        self.regs.sp = self.regs.sp.wrapping_add(1);
        self.val16 = u16::from_le_bytes([self.val8, hi]);
        self.command_cycle += 1;
      },
      3 => {
        self.regs.pc = self.val16;
        self.command_cycle += 1;
      },
      4 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }

  // fn rst(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
  //   match self.command_cycle {
  //     0 => {
  //       self.regs.sp = self.regs.sp.wrapping_sub(1);
  //       self.command_cycle += 1;
  //     },
  //     1 => {
  //       let [lo, hi] = u16::to_le_bytes(self.regs.pc);
  //       peripherals.write(self.regs.sp, hi);
  //       self.regs.sp = self.regs.sp.wrapping_sub(1);
  //       self.val8 = lo;
  //       self.command_cycle += 1;
  //     },
  //     2 => {
  //       peripherals.write(self.regs.sp, self.val8);
  //       self.command_cycle += 1;
  //     }
  //     3 => {
  //       self.prefetch(interrupts, peripherals, );
  //     },
  //     _ => panic!("Unexpected error."),
  //   }
  // }

  fn nop(&mut self, interrupts: &mut interrupts::Interrupts, peripherals: &mut peripherals::Peripherals) {
    match self.command_cycle {
      0 => {
        self.prefetch(interrupts, peripherals, self.regs.pc);
      },
      _ => panic!("Unexpected error."),
    }
  }
}
