use anyhow::{Result, ensure};
use log::info;
use std::{
  thread,
  time
};

use crate::cartridge;
use crate::cpu;
use crate::ppu;

const CPU_SPEED_HZ: u64 = 4_194_304;
const M_CYCLE_CLOCK: u64 = 4;

pub struct GameBoy {
  cpu: cpu::Cpu,
  ppu: ppu::Ppu,
}

impl GameBoy {
  pub fn new(cartridge: cartridge::Cartridge) -> Self {
    Self {
      cpu: cpu::Cpu::new(),
      ppu: ppu::Ppu::new(),
    }
  }

  pub fn run(&mut self) -> Result<()> {
    const M_CYCLE: time::Duration = time::Duration::from_nanos(
      M_CYCLE_CLOCK * 1_000_000_000 / CPU_SPEED_HZ
    );
    loop {
      let now = time::Instant::now();

      self.ppu.emulate_cycle();
      self.cpu.emulate_cycle();

      let elapsed = now.elapsed();
      if M_CYCLE > elapsed {
        thread::sleep(M_CYCLE - elapsed);
      }
    }
    // TODO: Event loop here.
  }
}
