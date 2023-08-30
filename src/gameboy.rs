use std::{
  cell::RefCell,
  fs::File,
  io::Write,
  rc::Rc,
  time,
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

fn key2joy(keycode: Keycode) -> Option<joypad::Button> {
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
  peripherals: Rc<RefCell<peripherals::Peripherals>>,

  lcd: lcd::LCD,
  sdl: Sdl,
}

impl GameBoy {
  pub fn new(bootrom: bootrom::Bootrom, cartridge: cartridge::Cartridge) -> Self {
    let sdl = sdl2::init().expect("failed to initialize SDL");
    let lcd = lcd::LCD::new(&sdl, 4);
    let audio = audio::Audio::new(&sdl);
    let peripherals = Rc::new(RefCell::new(peripherals::Peripherals::new(bootrom, cartridge, audio)));
    Self {
      cpu: cpu::Cpu::new(peripherals.clone()),
      interrupts: interrupts::Interrupts::new(),
      peripherals,

      lcd,
      sdl,
    }
  }

  pub fn run(&mut self) -> Result<()> {
    let mut event_pump = self.sdl.event_pump().unwrap();

    const M_CYCLE_NANOS: u128 = M_CYCLE_CLOCK * 1_000_000_000 / CPU_SPEED_HZ;
    let time = time::Instant::now();
    let mut elapsed = 0;
    'running: loop {
      let e = time.elapsed().as_nanos();
      for _ in 0..(e - elapsed) / M_CYCLE_NANOS {
        for event in event_pump.poll_iter() {
          match event {
            Event::Quit { .. } => break 'running,
            Event::Window { win_event: WindowEvent::Resized(w, h), .. } => self.lcd.resize(w as u32, h as u32),

            Event::KeyDown { keycode: Some(k), .. } => {
              if k == Keycode::Escape { break 'running }
              if k == Keycode::S { self.save_to_file() }
              key2joy(k).map(|j| self.peripherals.borrow_mut().joypad.button_down(&mut self.interrupts, j));
            },
            Event::KeyUp { keycode: Some(k), .. } => {
              key2joy(k).map(|j| self.peripherals.borrow_mut().joypad.button_up(j));
            },
            _ => (),
          }
        }
        self.cpu.emulate_cycle(&mut self.interrupts);
        if self.peripherals.borrow_mut().emulate_cycle(&mut self.interrupts) {
          self.lcd.draw(&self.peripherals.borrow().ppu.pixel_buffer);
        }
        elapsed += M_CYCLE_NANOS;
      }
    }
    Ok(())
  }

  fn save_to_file(&self) {
    if self.peripherals.borrow().cartridge.ram.len() == 0 {
      return warn!("The cartridge doesn't have ram.");
    }
    let fname = format!("{}-{}",
      self.peripherals.borrow().cartridge.title,
      Tokyo.from_utc_datetime(&Utc::now().naive_utc()).format("%Y_%m_%d_%H%M%S.sav"),
    );
    let mut file = if let Ok(f) = File::create(&fname) {
      f
    } else {
      return warn!("Cannot create save file \"{}\"", fname);
    };
    if file.write_all(&self.peripherals.borrow().cartridge.ram).is_err() {
      return warn!("Failed to save \"{}\"", fname);
    }
    if file.flush().is_err() {
      return warn!("Failed to save \"{}\"", fname);
    }
    info!("Save file \"{}\"", fname);
  }
}
