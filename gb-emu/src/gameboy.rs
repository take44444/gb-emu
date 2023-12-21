use serde::{Deserialize, Serialize};

use crate::{
  bootrom::Bootrom,
  cartridge::Cartridge,
  cpu::Cpu,
  peripherals::Peripherals,
};


#[derive(Clone, Serialize, Deserialize)]
pub struct GameBoy {
  pub cpu: Cpu,
  pub peripherals: Peripherals,
}

impl GameBoy {
  pub fn new(cart_rom: &[u8], save: &[u8]) -> Self {
    let cartridge = Cartridge::new(cart_rom.into(), if save.len() > 0 {
      Some(save.to_vec())
    } else {
      None
    });
    let is_cgb = cartridge.is_cgb;
    let bootrom = Bootrom::new();
    let peripherals = Peripherals::new(bootrom, cartridge, is_cgb);
    let cpu = Cpu::new();
    Self {
      cpu,
      peripherals,
    }
  }

  pub fn emulate_cycle(&mut self) -> bool {
    self.cpu.emulate_cycle(&mut self.peripherals);
    self.peripherals.timer.emulate_cycle(&mut self.cpu.interrupts);
    self.peripherals.serial.emulate_cycle(&mut self.cpu.interrupts);
    self.peripherals.apu.emulate_cycle();
    if let Some(addr) = self.peripherals.ppu.oam_dma {
      self.peripherals.ppu.oam_dma_emulate_cycle(self.peripherals.read(&self.cpu.interrupts, addr));
    }
    if let Some(_) = self.peripherals.ppu.hblank_dma {
      let mut src = [0; 0x10];
      for i in 0..0x10 {
        src[i as usize] = self.peripherals.read(&self.cpu.interrupts, self.peripherals.ppu.hdma_src + i);
      }
      self.peripherals.ppu.hblank_dma_emulate_cycle(src);
    }
    if let Some(len) = self.peripherals.ppu.general_dma {
      let mut src = Vec::new();
      for addr in self.peripherals.ppu.hdma_src..self.peripherals.ppu.hdma_src + len {
        src.push(self.peripherals.read(&self.cpu.interrupts, addr));
      }
      self.peripherals.ppu.general_dma_emulate_cycle(src);
    }
    self.peripherals.ppu.emulate_cycle(&mut self.cpu.interrupts)
  }
}
