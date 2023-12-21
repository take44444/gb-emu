use std::{
  env,
  fs::File,
  io::{ Read, Write, },
  process::exit,
  rc::Rc,
  time,
};

use sdl2::{
  event::{Event, WindowEvent},
  keyboard::Keycode,
  Sdl,
};

use gbemu::{
  gameboy,
  joypad,
};

use crate::{
  gameboy::GameBoy,
  lcd::LCD,
  joypad::Button,
  audio::Audio
};

mod lcd;
mod audio;

const CPU_CLOCK_HZ: u128 = 4_194_304;
const M_CYCLE_CLOCK: u128 = 4;
const M_CYCLE_NANOS: u128 = M_CYCLE_CLOCK * 1_000_000_000 / CPU_CLOCK_HZ;

fn key2joy(keycode: Keycode) -> Option<Button> {
  match keycode {
    Keycode::W    => Some(Button::Up),
    Keycode::S    => Some(Button::Down),
    Keycode::A    => Some(Button::Left),
    Keycode::D    => Some(Button::Right),
    Keycode::Num4 => Some(Button::Start),
    Keycode::Num3 => Some(Button::Select),
    Keycode::Num2 => Some(Button::B),
    Keycode::Num1 => Some(Button::A),
    _ => None,
  }
}

pub struct Emulator {
  gameboy: GameBoy,
  lcd: LCD,
  sdl: Sdl,
}

impl Emulator {
  pub fn new(cart_rom: &[u8], save: &[u8]) -> Self {
    let mut gameboy = GameBoy::new(cart_rom, save);
    let sdl = sdl2::init().expect("failed to initialize SDL");
    let lcd = LCD::new(&sdl, 4);
    let audio = Audio::new(&sdl);
    gameboy.peripherals.apu.set_callback(Rc::new(audio.0));
    Self {
      gameboy,
      lcd,
      sdl,
    }
  }

  pub fn run(&mut self) {
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
              key2joy(k).map(|j| self.gameboy.peripherals.joypad.button_down(&mut self.gameboy.cpu.interrupts, j));
            },
            Event::KeyUp { keycode: Some(k), .. } => {
              if k == Keycode::Return { self.save_to_file() }
              key2joy(k).map(|j| self.gameboy.peripherals.joypad.button_up(j));
            },
            _ => (),
          }
        }
        if self.gameboy.emulate_cycle() {
          self.lcd.draw(&self.gameboy.peripherals.ppu.buffer);
        }
        if self.gameboy.peripherals.serial.send().is_some() {
          self.gameboy.peripherals.serial.recv(0xFF);
        }
        elapsed += M_CYCLE_NANOS;
      }
    }
  }

  fn save_to_file(&self) {
    if self.gameboy.peripherals.cartridge.sram.len() == 0 {
      return eprintln!("The cartridge doesn't have ram.");
    }
    let fname = format!("{}.SAV", self.gameboy.peripherals.cartridge.title);
    let mut file = if let Ok(f) = File::create(&fname) {
      f
    } else {
      return;
    };
    if file.write_all(&self.gameboy.peripherals.cartridge.sram).is_err() {
      return eprintln!("Failed to save \"{}\"", fname);
    }
    if file.flush().is_err() {
      return eprintln!("Failed to save \"{}\"", fname);
    }
    println!("Save file \"{}\"", fname);
  }
}

fn file2vec(fname: &String) -> Vec<u8> {
  if let Ok(mut file) = File::open(fname) {
    let mut ret = vec![];
    file.read_to_end(&mut ret).unwrap();
    ret
  } else {
    panic!("Cannot open {}.", fname);
  }
}

fn main() {
  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    eprintln!("The file name argument is required.");
    exit(1);
  }
  let cartridge_raw = file2vec(&args[1]);
  let save = if args.len() >= 3 { file2vec(&args[2]) } else { vec![] };

  let mut emulator = Emulator::new(&cartridge_raw, &save);
  emulator.run();
}
