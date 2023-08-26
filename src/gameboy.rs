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
          Event::KeyDown {
            keycode: Some(keycode),
            ..
          } => {
            match keycode {
              Keycode::Up => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::Up),
              Keycode::Down => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::Down),
              Keycode::Left => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::Left),
              Keycode::Right => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::Right),
              Keycode::Num2 => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::Start),
              Keycode::Num1 => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::Select),
              Keycode::Backspace => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::B),
              Keycode::Return => self.peripherals.joypad.button_down(&mut self.interrupts, joypad::Button::A),
              Keycode::S => self.save_to_file(),
              Keycode::Escape => break 'running,
              _ => (),
            }
          },
          Event::KeyUp {
            keycode: Some(keycode),
            ..
          } => {
            match keycode {
              Keycode::Up => self.peripherals.joypad.button_up(joypad::Button::Up),
              Keycode::Down => self.peripherals.joypad.button_up(joypad::Button::Down),
              Keycode::Left => self.peripherals.joypad.button_up(joypad::Button::Left),
              Keycode::Right => self.peripherals.joypad.button_up(joypad::Button::Right),
              Keycode::Num2 => self.peripherals.joypad.button_up(joypad::Button::Start),
              Keycode::Num1 => self.peripherals.joypad.button_up(joypad::Button::Select),
              Keycode::Backspace => self.peripherals.joypad.button_up(joypad::Button::B),
              Keycode::Return => self.peripherals.joypad.button_up(joypad::Button::A),
              _ => (),
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
