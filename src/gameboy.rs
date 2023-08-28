use std::{
  fs::File,
  io::Write,
  time
};
use anyhow::Result;
use chrono::{TimeZone, Utc};
use chrono_tz::Asia::Tokyo;
use log::{info, warn};
use sdl2::{
  event::{Event, WindowEvent},
  keyboard::Keycode,
  Sdl,
};

use crate::{bootrom, cartridge, joypad, audio, lcd, interrupts, peripherals, cpu};

pub const CPU_SPEED_HZ: u128 = 4_194_304;
const M_CYCLE_CLOCK: u128 = 4;

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

  lcd: lcd::LCD,
  audio: audio::Audio,
  sdl: Sdl,
}

impl GameBoy {
  pub fn new(bootrom: bootrom::Bootrom, cartridge: cartridge::Cartridge) -> Self {
    let sdl = sdl2::init().expect("failed to initialize SDL");
    let lcd = lcd::LCD::new(&sdl, 4);
    let audio = audio::Audio::new(&sdl);
    Self {
      cpu: cpu::Cpu::new(),
      interrupts: interrupts::Interrupts::new(),
      peripherals: peripherals::Peripherals::new(bootrom, cartridge),

      lcd,
      audio,
      sdl,
    }
  }

  pub fn run(&mut self) -> Result<()> {
    let mut event_pump = self.sdl.event_pump().unwrap();

    const M_CYCLE: u128 = M_CYCLE_CLOCK * 1_000_000_000 / CPU_SPEED_HZ;
    let mut time = time::Instant::now();
    let mut elapsed = 0;
    'running: loop {
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
          Event::Window {
            win_event: WindowEvent::Resized(width, height),
            ..
          } => {
            self.lcd.resize(width as u32, height as u32);
          },
          _ => (),
        }
      }
      self.audio.play(self.peripherals.apu.front_buffer.as_ref());

      let e = time.elapsed().as_nanos();
      if elapsed + e > M_CYCLE {
        elapsed += e;
        time = time::Instant::now();
        for _ in 0..elapsed / M_CYCLE {
          self.cpu.emulate_cycle(&mut self.interrupts, &mut self.peripherals);
          if self.peripherals.emulate_cycle(&mut self.interrupts) {
            self.lcd.draw(&self.peripherals.ppu.pixel_buffer);
          }
        }
        elapsed %= M_CYCLE;
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
