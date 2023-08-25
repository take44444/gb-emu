use anyhow::Result;
use sdl2::{
  event::Event,
  keyboard::Keycode,
};
use std::{
  thread,
  time
};

use crate::{cartridge, bootrom, lcd};
use crate::cpu;
use crate::interrupts;
use crate::peripherals;

const CPU_SPEED_HZ: u64 = 4_194_304;
const M_CYCLE_CLOCK: u64 = 4;

pub struct GameBoy {
  cpu: cpu::Cpu,
  interrupts: interrupts::Interrupts,
  peripherals: peripherals::Peripherals,
}

impl GameBoy {
  pub fn new(bootrom: bootrom::Bootrom, cartridge: cartridge::Cartridge) -> Self {
    Self {
      cpu: cpu::Cpu::new(),
      interrupts: interrupts::Interrupts::new(),
      peripherals: peripherals::Peripherals::new(bootrom, cartridge),
    }
  }

  pub fn run(&mut self) -> Result<()> {
    let sdl_context = sdl2::init().unwrap();
    let mut lcd = lcd::LCD::new(&sdl_context, 4);
    let mut event_pump = sdl_context.event_pump().unwrap();

    const M_CYCLE: time::Duration = time::Duration::from_nanos(
      M_CYCLE_CLOCK * 1_000_000_000 / CPU_SPEED_HZ
    );
    let mut bef = time::Instant::now();
    let mut cycles = 0;
    'running: loop {
      self.cpu.emulate_cycle(&mut self.interrupts, &mut self.peripherals);
      cycles += 1;
      if self.peripherals.emulate_cycle(&mut self.interrupts) {
        let elapsed = bef.elapsed();
        if M_CYCLE * cycles > elapsed {
          thread::sleep(M_CYCLE * cycles - elapsed);
        }
        bef = time::Instant::now();
        cycles = 0;
        lcd.draw(&self.peripherals.ppu.pixel_buffer)
      }

      for event in event_pump.poll_iter() {
        match event {
          Event::Quit { .. }
          | Event::KeyDown {
            keycode: Some(Keycode::Escape),
            ..
          } => break 'running,
          _ => (),
        }
      }
    }
    Ok(())
  }
}
