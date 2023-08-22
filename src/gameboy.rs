use anyhow::{
  Result, ensure
};
use log::info;
use sdl2::{
  event::Event,
  keyboard::Keycode,
  pixels::PixelFormatEnum,
  render::{
    Texture, WindowCanvas
  }
};
use std::{
  thread,
  time
};

use crate::{cartridge, bootrom};
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
      peripherals: peripherals::Peripherals::new(),
    }
  }

  pub fn run(&mut self) -> Result<()> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
      .window("gb-emu", 320, 288)
      .position_centered()
      .build()
      .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
      .create_texture_streaming(PixelFormatEnum::RGB24, 160, 144)
      .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    const M_CYCLE: time::Duration = time::Duration::from_nanos(
      M_CYCLE_CLOCK * 1_000_000_000 / CPU_SPEED_HZ
    );
    'running: loop {
      let now = time::Instant::now();

      // self.cpu.emulate_cycle(&mut self.interrupts, &mut self.peripherals);

      if self.peripherals.emulate_cycle(&mut self.interrupts) {
        texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
          for y in 0..144 {
            for x in 0..160 {
              let offset = y * pitch + x * 3;
              let color = self.peripherals.ppu.pixel_buffer[y * 160 + x] as u8;

              buf[offset] = color;
              buf[offset + 1] = color;
              buf[offset + 2] = color;
            }
          }
        }).unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
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

      let elapsed = now.elapsed();
      if M_CYCLE > elapsed {
        thread::sleep(M_CYCLE - elapsed);
      }
    }
    Ok(())
  }
}
