use std::{
  cell::RefCell,
  rc::Rc,
  time,
};

use sdl2::{
  event::{Event, WindowEvent},
  keyboard::Keycode,
  Sdl,
};

use crate::{
  bootrom::Bootrom,
  cartridge::Cartridge,
  cpu::{
    Cpu,
    interrupts::Interrupts,
  },
  peripherals::Peripherals,
  lcd::LCD,
  joypad::Button,
  audio::Audio
};

pub const CPU_CLOCK_HZ: u128 = 4_194_304;
const M_CYCLE_CLOCK: u128 = 4;

fn key2joy(keycode: Keycode) -> Option<Button> {
  match keycode {
    Keycode::Up => Some(Button::Up),
    Keycode::Down => Some(Button::Down),
    Keycode::Left => Some(Button::Left),
    Keycode::Right => Some(Button::Right),
    Keycode::Num2 => Some(Button::Start),
    Keycode::Num1 => Some(Button::Select),
    Keycode::Backspace => Some(Button::B),
    Keycode::Return => Some(Button::A),
    _ => None,
  }
}
pub struct GameBoy {
  cpu: Cpu,
  peripherals: Peripherals,
  lcd: LCD,
  sdl: Sdl,
}

impl GameBoy {
  pub fn new(bootrom: Bootrom, cartridge: Cartridge) -> Self {
    let sdl = sdl2::init().expect("failed to initialize SDL");
    let lcd = LCD::new(&sdl, 4);
    let audio = Audio::new(&sdl);
    let interrupts = Rc::new(RefCell::new(Interrupts::default()));
    let peripherals = Peripherals::new(bootrom, cartridge, audio, interrupts.clone());
    let cpu = Cpu::new(interrupts);
    Self {
      cpu,
      peripherals,
      lcd,
      sdl,
    }
  }

  pub fn run(&mut self) {
    const M_CYCLE_NANOS: u128 = M_CYCLE_CLOCK * 1_000_000_000 / CPU_CLOCK_HZ;
    let mut event_pump = self.sdl.event_pump().unwrap();
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
              // if k == Keycode::S { self.save_to_file() }
              key2joy(k).map(|j| self.peripherals.joypad.button_down(j));
            },
            Event::KeyUp { keycode: Some(k), .. } => {
              key2joy(k).map(|j| self.peripherals.joypad.button_up(j));
            },
            _ => (),
          }
        }
        self.cpu.emulate_cycle(&mut self.peripherals);
        if self.peripherals.emulate_cycle() {
          self.lcd.draw(self.peripherals.ppu.pixel_buffer());
        }
        elapsed += M_CYCLE_NANOS;
      }
    }
  }
}
