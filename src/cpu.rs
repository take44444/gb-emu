use crate::bus;
use crate::register;

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

struct Context {
  val8: u8,
  val16: u16,
}

pub struct Cpu {
  regs: register::Registers,
  opcode: u8,
  command_cycle: u8,
  ctx: Context,
}

impl Cpu {
  pub fn new() -> Self {
    Self {
      regs: register::Registers::new(),
      opcode: 0x00,
      command_cycle: 0,
      ctx: Context {
        val8: 0,
        val16: 0,
      },
    }
  }

  pub fn emulate_cycle(&mut self, bus: &mut bus::Bus) {
    match self.opcode {
      0x7f => self.ld_r_r(bus, Reg8::A, Reg8::A),
      0x78 => self.ld_r_r(bus, Reg8::A, Reg8::B),
      0x79 => self.ld_r_r(bus, Reg8::A, Reg8::C),
      0x7a => self.ld_r_r(bus, Reg8::A, Reg8::D),
      0x7b => self.ld_r_r(bus, Reg8::A, Reg8::E),
      0x7c => self.ld_r_r(bus, Reg8::A, Reg8::H),
      0x7d => self.ld_r_r(bus, Reg8::A, Reg8::L),
      0x47 => self.ld_r_r(bus, Reg8::B, Reg8::A),
      0x40 => self.ld_r_r(bus, Reg8::B, Reg8::B),
      0x41 => self.ld_r_r(bus, Reg8::B, Reg8::C),
      0x42 => self.ld_r_r(bus, Reg8::B, Reg8::D),
      0x43 => self.ld_r_r(bus, Reg8::B, Reg8::E),
      0x44 => self.ld_r_r(bus, Reg8::B, Reg8::H),
      0x45 => self.ld_r_r(bus, Reg8::B, Reg8::L),
      0x4f => self.ld_r_r(bus, Reg8::C, Reg8::A),
      0x48 => self.ld_r_r(bus, Reg8::C, Reg8::B),
      0x49 => self.ld_r_r(bus, Reg8::C, Reg8::C),
      0x4a => self.ld_r_r(bus, Reg8::C, Reg8::D),
      0x4b => self.ld_r_r(bus, Reg8::C, Reg8::E),
      0x4c => self.ld_r_r(bus, Reg8::C, Reg8::H),
      0x4d => self.ld_r_r(bus, Reg8::C, Reg8::L),
      0x57 => self.ld_r_r(bus, Reg8::D, Reg8::A),
      0x50 => self.ld_r_r(bus, Reg8::D, Reg8::B),
      0x51 => self.ld_r_r(bus, Reg8::D, Reg8::C),
      0x52 => self.ld_r_r(bus, Reg8::D, Reg8::D),
      0x53 => self.ld_r_r(bus, Reg8::D, Reg8::E),
      0x54 => self.ld_r_r(bus, Reg8::D, Reg8::H),
      0x55 => self.ld_r_r(bus, Reg8::D, Reg8::L),
      0x5f => self.ld_r_r(bus, Reg8::E, Reg8::A),
      0x58 => self.ld_r_r(bus, Reg8::E, Reg8::B),
      0x59 => self.ld_r_r(bus, Reg8::E, Reg8::C),
      0x5a => self.ld_r_r(bus, Reg8::E, Reg8::D),
      0x5b => self.ld_r_r(bus, Reg8::E, Reg8::E),
      0x5c => self.ld_r_r(bus, Reg8::E, Reg8::H),
      0x5d => self.ld_r_r(bus, Reg8::E, Reg8::L),
      0x67 => self.ld_r_r(bus, Reg8::H, Reg8::A),
      0x60 => self.ld_r_r(bus, Reg8::H, Reg8::B),
      0x61 => self.ld_r_r(bus, Reg8::H, Reg8::C),
      0x62 => self.ld_r_r(bus, Reg8::H, Reg8::D),
      0x63 => self.ld_r_r(bus, Reg8::H, Reg8::E),
      0x64 => self.ld_r_r(bus, Reg8::H, Reg8::H),
      0x65 => self.ld_r_r(bus, Reg8::H, Reg8::L),
      0x6f => self.ld_r_r(bus, Reg8::L, Reg8::A),
      0x68 => self.ld_r_r(bus, Reg8::L, Reg8::B),
      0x69 => self.ld_r_r(bus, Reg8::L, Reg8::C),
      0x6a => self.ld_r_r(bus, Reg8::L, Reg8::D),
      0x6b => self.ld_r_r(bus, Reg8::L, Reg8::E),
      0x6c => self.ld_r_r(bus, Reg8::L, Reg8::H),
      0x6d => self.ld_r_r(bus, Reg8::L, Reg8::L),
      0x3e => self.ld_r_n(bus, Reg8::A),
      0x06 => self.ld_r_n(bus, Reg8::B),
      0x0e => self.ld_r_n(bus, Reg8::C),
      0x16 => self.ld_r_n(bus, Reg8::D),
      0x1e => self.ld_r_n(bus, Reg8::E),
      0x26 => self.ld_r_n(bus, Reg8::H),
      0x2e => self.ld_r_n(bus, Reg8::L),
      0x7e => self.ld_r_i(bus, Reg8::A, Indirect::HL),
      0x46 => self.ld_r_i(bus, Reg8::B, Indirect::HL),
      0x4e => self.ld_r_i(bus, Reg8::C, Indirect::HL),
      0x56 => self.ld_r_i(bus, Reg8::D, Indirect::HL),
      0x5e => self.ld_r_i(bus, Reg8::E, Indirect::HL),
      0x66 => self.ld_r_i(bus, Reg8::H, Indirect::HL),
      0x6e => self.ld_r_i(bus, Reg8::L, Indirect::HL),
      0x77 => self.ld_i_r(bus, Indirect::HL, Reg8::A),
      0x70 => self.ld_i_r(bus, Indirect::HL, Reg8::B),
      0x71 => self.ld_i_r(bus, Indirect::HL, Reg8::C),
      0x72 => self.ld_i_r(bus, Indirect::HL, Reg8::D),
      0x73 => self.ld_i_r(bus, Indirect::HL, Reg8::E),
      0x74 => self.ld_i_r(bus, Indirect::HL, Reg8::H),
      0x75 => self.ld_i_r(bus, Indirect::HL, Reg8::L),
      0x36 => self.ld_i_n(bus, Indirect::HL),
      0x0a => self.ld_r_i(bus, Reg8::A, Indirect::BC),
      0x1a => self.ld_r_i(bus, Reg8::A, Indirect::DE),
      0x02 => self.ld_i_r(bus, Indirect::BC, Reg8::A),
      0x12 => self.ld_i_r(bus, Indirect::DE, Reg8::A),
      0xfa => self.ld_r_d(bus, Reg8::A),
      0xea => self.ld_d_r(bus, Reg8::A),
      0xf2 => self.ld_r_i(bus, Reg8::A, Indirect::CFF),
      0xe2 => self.ld_i_r(bus, Indirect::CFF, Reg8::A),
      0xf0 => self.ldh_r_d(bus, Reg8::A),
      0xe0 => self.ldh_d_r(bus, Reg8::A),
      0x3a => self.ld_r_i(bus, Reg8::A, Indirect::HLD),
      0x32 => self.ld_i_r(bus, Indirect::HLD, Reg8::A),
      0x2a => self.ld_r_i(bus, Reg8::A, Indirect::HLI),
      0x22 => self.ld_i_r(bus, Indirect::HLI, Reg8::A),

      0x00 => self.nop(bus),
      _ => panic!("Undefined opcode {}", self.opcode),
    }
  }

  fn ld_r_r(&mut self, bus: &mut bus::Bus, dst: Reg8, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.write_r(dst, self.read_r(src));
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r_n(&mut self, bus: &mut bus::Bus, dst: Reg8) {
    match self.command_cycle {
      0 => {
        let data = self.read_imm(bus);
        self.write_r(dst, data);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r_i(&mut self, bus: &mut bus::Bus, dst: Reg8, src: Indirect) {
    match self.command_cycle {
      0 => {
        let data = self.read_i(src, bus);
        self.write_r(dst, data);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_i_r(&mut self, bus: &mut bus::Bus, dst: Indirect, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.write_i(dst, self.read_r(src), bus);
        self.command_cycle += 1;
      },
      1 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_i_n(&mut self, bus: &mut bus::Bus, dst: Indirect) {
    match self.command_cycle {
      0 => {
        self.ctx.val8 = self.read_imm(bus);
        self.command_cycle += 1;
      },
      1 => {
        self.write_i(dst, self.ctx.val8, bus);
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_r_d(&mut self, bus: &mut bus::Bus, dst: Reg8) {
    match self.command_cycle {
      0 => {
        self.ctx.val8 = self.read_imm(bus);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.ctx.val8;
        let hi = self.read_imm(bus);
        self.ctx.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        self.write_r(dst, bus.read_bus(self.ctx.val16));
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ld_d_r(&mut self, bus: &mut bus::Bus, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.ctx.val8 = self.read_imm(bus);
        self.command_cycle += 1;
      },
      1 => {
        let lo = self.ctx.val8;
        let hi = self.read_imm(bus);
        self.ctx.val16 = u16::from_le_bytes([lo, hi]);
        self.command_cycle += 1;
      },
      2 => {
        bus.write_bus(self.ctx.val16, self.read_r(src));
        self.command_cycle += 1;
      },
      3 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ldh_r_d(&mut self, bus: &mut bus::Bus, dst: Reg8) {
    match self.command_cycle {
      0 => {
        self.ctx.val8 = self.read_imm(bus);
        self.command_cycle += 1;
      },
      1 => {
        self.write_r(dst, bus.read_bus(0xff00 | (self.ctx.val8 as u16)));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn ldh_d_r(&mut self, bus: &mut bus::Bus, src: Reg8) {
    match self.command_cycle {
      0 => {
        self.ctx.val8 = self.read_imm(bus);
        self.command_cycle += 1;
      },
      1 => {
        bus.write_bus(0xff00 | (self.ctx.val8 as u16), self.read_r(src));
        self.command_cycle += 1;
      },
      2 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn nop(&mut self, bus: &mut bus::Bus) {
    match self.command_cycle {
      0 => {
        self.prefetch_next(bus);
      },
      _ => panic!("Unexpected error."),
    }
  }

  fn prefetch_next(&mut self, bus: &mut bus::Bus) {
    self.opcode = self.read_imm(bus);
    self.command_cycle = 0;
  }

  fn read_r(&self, src: Reg8) -> u8 {
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

  // fn read_rr(&self, src: Reg16) -> u8 {
  //   match src {
  //     Reg16::BC => self.regs.bc(),
  //     Reg16::DE => self.regs.de(),
  //     Reg16::HL => self.regs.hl(),
  //     _ => panic!("Unexpected error."),
  //   }
  // }

  fn write_rr(&mut self, dst: Reg16, data: u16) {
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

  fn read_i(&mut self, src: Indirect, bus: &mut bus::Bus) -> u8 {
    match src {
      Indirect::BC => bus.read_bus(self.regs.bc()),
      Indirect::DE => bus.read_bus(self.regs.de()),
      Indirect::HL => bus.read_bus(self.regs.hl()),
      Indirect::CFF => bus.read_bus(0xff00 | (self.regs.c as u16)),
      Indirect::HLD => {
        let addr = self.regs.hl();
        let ret = bus.read_bus(addr);
        self.write_rr(Reg16::HL, addr.wrapping_sub(1));
        ret
      },
      Indirect::HLI => {
        let addr = self.regs.hl();
        let ret = bus.read_bus(addr);
        self.write_rr(Reg16::HL, addr.wrapping_add(1));
        ret
      },
    }
  }

  fn read_imm(&mut self, bus: &mut bus::Bus) -> u8 {
    let ret = bus.read_bus(self.regs.pc);
    self.regs.pc = self.regs.pc.wrapping_add(1);
    ret
  }

  fn write_r(&mut self, dst: Reg8, data: u8) {
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

  fn write_i(&mut self, dst: Indirect, data: u8, bus: &mut bus::Bus) {
    match dst {
      Indirect::BC => bus.write_bus(self.regs.bc(), data),
      Indirect::DE => bus.write_bus(self.regs.de(), data),
      Indirect::HL => bus.write_bus(self.regs.hl(), data),
      Indirect::CFF => bus.write_bus(0xff00 | (self.regs.c as u16), data),
      Indirect::HLD => {
        let addr = self.regs.hl();
        bus.write_bus(addr, data);
        self.write_rr(Reg16::HL, addr.wrapping_sub(1));
      },
      Indirect::HLI => {
        let addr = self.regs.hl();
        bus.write_bus(addr, data);
        self.write_rr(Reg16::HL, addr.wrapping_add(1));
      },
    }
  }
}
