use std::{
  fs::File,
  io::Write,
  thread,
  time
};
use anyhow::Result;
use sdl2::{
  event::Event,
  keyboard::Keycode,
};
use log::{info, warn};
use chrono::{TimeZone, Utc};
use chrono_tz::Asia::Tokyo;

use crate::{cartridge, bootrom, lcd, joypad};
use crate::cpu;
use crate::interrupts;
use crate::peripherals;

const CPU_SPEED_HZ: u64 = 4_194_304;
const M_CYCLE_CLOCK: u64 = 4;

fn map_key2joy(keycode: Keycode) -> Option<joypad::Button> {
  match keycode {
    Keycode::Up => Some(joypad::Button::Up),
    Keycode::Down => Some(joypad::Button::Down),
    Keycode::Left => Some(joypad::Button::Left),
    Keycode::Right => Some(joypad::Button::Right),
    Keycode::Num2 => Some(joypad::Button::Start),
    Keycode::Num1 => Some(joypad::Button::Select),
    Keycode::Backspace => Some(joypad::Button::B),
    Keycode::Return => Some(joypad::Button::A),
    _ => None,
  }
}

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
        lcd.draw(&self.peripherals.ppu.pixel_buffer);
      }

      for event in event_pump.poll_iter() {
        match event {
          Event::KeyDown {
            keycode: Some(keycode),
            ..
          } => {
            match keycode {
              Keycode::S => self.save_to_file(),
              Keycode::Escape => break 'running,
              keycode => if let Some(joycode) = map_key2joy(keycode) {
                self.peripherals.joypad.button_down(&mut self.interrupts, joycode);
              },
            }
          },
          Event::KeyUp {
            keycode: Some(keycode),
            ..
          } => {
            if let Some(joycode) = map_key2joy(keycode) {
              self.peripherals.joypad.button_up(joycode);
            }
          },
          Event::Quit { .. } => break 'running,
          _ => (),
        }
      }
    }
    Ok(())
  }

  fn save_to_file(&self) {
    if self.peripherals.cartridge.ram.len() == 0 {
      return warn!("The cartridge doesn't have ram.");
    }
    let fname = format!("{}-{}",
      self.peripherals.cartridge.title,
      Tokyo.from_utc_datetime(&Utc::now().naive_utc()).format("%Y_%m_%d_%H%M%S.sav"),
    );
    let mut file = if let Ok(f) = File::create(&fname) {
      f
    } else {
      return warn!("Cannot create save file \"{}\"", fname);
    };
    if let Err(_) = file.write_all(&self.peripherals.cartridge.ram) {
      return warn!("Faile to save \"{}\"", fname);
    }
    if let Err(_) = file.flush() {
      return warn!("Faile to save \"{}\"", fname);
    }
    info!("Save file \"{}\"", fname);
  }
}
