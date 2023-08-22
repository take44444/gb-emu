use crate::interrupts;
use crate::wram;
use crate::hram;
use crate::ppu;

pub struct Peripherals {
  wram: wram::WRam,
  hram: hram::HRam,
  pub ppu: ppu::Ppu,
  // timer: timer::Timer,
  // apu: apu::Apu,
}

impl Peripherals {
  pub fn new() -> Self {
    Self {
      wram: wram::WRam::new(),
      hram: hram::HRam::new(),
      ppu: ppu::Ppu::new(),
    }
  }

  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) -> bool {
    // self.emulate_oam_dma_cycle();
    let ret = self.ppu.emulate_cycle(interrupts);
    // self.timer.emulate_cycle();
    // self.apu.emulate_cycle();
    ret
  }

  pub fn read(&self, interrupts: &interrupts::Interrupts, addr: u16) -> u8 {
    match (addr >> 8) as u8 {
      0x80..=0x9F => self.ppu.read_vram(addr),
      0xC0..=0xDF => self.wram.read(addr),
      // ECHO RAM
      0xE0..=0xFD => self.wram.read(addr),
      0xFE => {
        match addr as u8 {
          0x00..=0x9F => {
            self.ppu.read_oam(addr)
          }
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      },
      0xFF => {
        match addr as u8 {
          0x40 => self.ppu.get_lcdc(),
          0x41 => self.ppu.get_stat(),
          0x42 => self.ppu.get_scy(),
          0x43 => self.ppu.get_scx(),
          0x44 => self.ppu.get_ly(),
          0x45 => self.ppu.get_lyc(),
          0x47 => self.ppu.get_bgp(),
          0x48 => self.ppu.get_obp0(),
          0x49 => self.ppu.get_obp1(),
          0x4A => self.ppu.get_wy(),
          0x4B => self.ppu.get_wx(),
          0x80..=0xFE => self.hram.read(addr),
          0xFF => interrupts.intr_enable,
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      },
      _ => panic!("Unsupported read at ${:04x}", addr),
    }
  }

  pub fn write(&mut self, interrupts: &mut interrupts::Interrupts, addr: u16, val: u8) {
    match (addr >> 8) as u8 {
      0x80..=0x9F => self.ppu.write_vram(addr, val),
      0xC0..=0xDF => self.wram.write(addr, val),
      // ECHO RAM
      0xE0..=0xFD => self.wram.write(addr, val),
      0xFE => {
        match addr as u8 {
          0x00..=0x9F => {
            self.ppu.write_oam(addr, val)
          }
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      },
      0xFF => {
        match addr as u8 {
          0x40 => self.ppu.set_lcdc(val),
          0x41 => self.ppu.set_stat(val),
          0x42 => self.ppu.set_scy(val),
          0x43 => self.ppu.set_scx(val),
          0x44 => self.ppu.reset_ly(),
          0x45 => self.ppu.set_lyc(val),
          0x47 => self.ppu.set_bgp(val),
          0x48 => self.ppu.set_obp0(val),
          0x49 => self.ppu.set_obp1(val),
          0x4A => self.ppu.set_wy(val),
          0x4B => self.ppu.set_wx(val),
          0x80..=0xFE => self.hram.write(addr, val),
          0xFF => interrupts.intr_enable = val,
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      }
      _ => panic!("Unsupported read at ${:04x}", addr),
    }
  }
}