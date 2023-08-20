use crate::ppu;

pub struct Peripherals {
  ppu: ppu::Ppu,
  // timer: timer::Timer,
  // apu: apu::Apu,
}

impl Peripherals {
  pub fn new() -> Self {
    Self {
      ppu: ppu::Ppu::new(),
    }
  }

  pub fn emulate_cycle(&mut self) {
    // self.emulate_oam_dma_cycle();
    self.ppu.emulate_cycle();
    // self.timer.emulate_cycle();
    // self.apu.emulate_cycle();
  }

  pub fn read(&self, addr: u16) -> u8 {
    0
  }

  pub fn write(&mut self, addr: u16, data: u8) {

  }
}