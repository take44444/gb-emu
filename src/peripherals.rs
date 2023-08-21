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
      0xC0..=0xDF => self.wram.read(addr),
      // ECHO RAM
      0xE0..=0xFD => self.wram.read(addr),
      0xFF => {
        match addr as u8 {
          0x80..=0xFE => self.hram.read(addr),
          0xFF => interrupts.intr_enable,
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      }
      _ => panic!("Unsupported read at ${:04x}", addr),
    }
  }

  pub fn write(&mut self, interrupts: &mut interrupts::Interrupts, addr: u16, val: u8) {
    match (addr >> 8) as u8 {
      0xC0..=0xDF => self.wram.write(addr, val),
      // ECHO RAM
      0xE0..=0xFD => self.wram.write(addr, val),
      0xFF => {
        match addr as u8 {
          0x80..=0xFE => self.hram.write(addr, val),
          0xFF => interrupts.intr_enable = val,
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      }
      _ => panic!("Unsupported read at ${:04x}", addr),
    }
  }
}