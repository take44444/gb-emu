use std::{time, fs::File, io::Write, rc::Rc};

use sdl2::{
  event::{Event, WindowEvent},
  keyboard::Keycode,
  Sdl,
};

use crate::{
  bootrom::Bootrom,
  cartridge::Cartridge,
  cpu::Cpu,
  peripherals::Peripherals,
  lcd::LCD,
  joypad::Button,
  audio::Audio
};

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
    let mut peripherals = Peripherals::new(bootrom, cartridge, true);
    peripherals.apu.set_callback(Rc::new(audio.0));
    let cpu = Cpu::new();
    Self {
      cpu,
      peripherals,
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
              key2joy(k).map(|j| self.peripherals.joypad.button_down(&mut self.cpu.interrupts, j));
            },
            Event::KeyUp { keycode: Some(k), .. } => {
              if k == Keycode::Return { self.save_to_file() }
              key2joy(k).map(|j| self.peripherals.joypad.button_up(j));
            },
            _ => (),
          }
        }
        self.cpu.emulate_cycle(&mut self.peripherals);
        self.peripherals.timer.emulate_cycle(&mut self.cpu.interrupts);
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
        if self.peripherals.ppu.emulate_cycle(&mut self.cpu.interrupts) {
          self.lcd.draw(&self.peripherals.ppu.pixel_buffer());
        }
        elapsed += M_CYCLE_NANOS;
      }
    }
  }

  fn save_to_file(&self) {
    if self.peripherals.cartridge.sram.len() == 0 {
      return eprintln!("The cartridge doesn't have ram.");
    }
    let fname = format!("{}.SAV", self.peripherals.cartridge.title);
    let mut file = if let Ok(f) = File::create(&fname) {
      f
    } else {
      return;
    };
    if file.write_all(&self.peripherals.cartridge.sram).is_err() {
      return eprintln!("Failed to save \"{}\"", fname);
    }
    if file.flush().is_err() {
      return eprintln!("Failed to save \"{}\"", fname);
    }
    println!("Save file \"{}\"", fname);
  }
}
