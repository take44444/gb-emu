use std::rc::Rc;

use serde::{Deserialize, Serialize};
use js_sys::{Float32Array, Function, Uint8ClampedArray, Uint8Array};
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
// extern "C" {
//   #[wasm_bindgen(js_namespace = console)]
//   fn log(s: &str);
// }
// macro_rules! console_log {
//   ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
// }

use gbemu::{
  gameboy::GameBoy,
  joypad::Button,
  SAMPLE_RATE,
};

fn key2joy(keycode: &str) -> Option<Button> {
  match keycode {
    "KeyW"      => Some(Button::Up),
    "KeyS"      => Some(Button::Down),
    "KeyA"      => Some(Button::Left),
    "KeyD"      => Some(Button::Right),
    "Digit4"    => Some(Button::Start),
    "Digit3"    => Some(Button::Select),
    "Digit2"    => Some(Button::B),
    "Digit1"    => Some(Button::A),
    _           => None,
  }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct GameBoyHandle {
  gameboy: GameBoy,
  gameboy2: Option<GameBoy>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
  cycle: u32,
  down: bool,
  code: String,
}

#[wasm_bindgen]
impl GameBoyHandle {
  pub fn new(cart_rom: &[u8], save: &[u8]) -> Self {
    console_error_panic_hook::set_once();
    Self {
      gameboy: GameBoy::new(cart_rom, save),
      gameboy2: None,
    }
  }

  pub fn set_apu_callback(&mut self, callback: Function) {
    self.gameboy.peripherals.apu.set_callback(Rc::new(move |buffer: &[f32]| {
      callback
        .call1(&JsValue::null(), &Float32Array::from(buffer))
        .unwrap();
    }));
  }

  pub fn title(&self) -> String {
    self.gameboy.peripherals.cartridge.title.clone()
  }

  pub fn save(&self) -> Uint8Array {
    Uint8Array::from(self.gameboy.peripherals.cartridge.sram.as_ref())
  }

  pub fn to_json(&self) -> String {
    serde_json::to_string(&self.gameboy).unwrap()
  }

  pub fn _clone(&self) -> Self {
    self.clone()
  }

  pub fn connect(&mut self, json: String) {
    self.gameboy2 = serde_json::from_str(&json).ok();
  }

  pub fn disconnect(&mut self) {
    self.gameboy2 = None;
  }

  pub fn emulate_cycle(&mut self) -> bool {
    let ret = self.gameboy.emulate_cycle();
    match self.gameboy2.as_mut() {
      Some(gb) => {
        gb.emulate_cycle();
        if let Some(data) = gb.peripherals.serial.send() {
          gb.peripherals.serial.recv(self.gameboy.peripherals.serial.data);
          self.gameboy.peripherals.serial.recv(data);
        }
        if let Some(data) = self.gameboy.peripherals.serial.send() {
          self.gameboy.peripherals.serial.recv(gb.peripherals.serial.data);
          gb.peripherals.serial.recv(data);
        }
      },
      None => if self.gameboy.peripherals.serial.send().is_some() {
        self.gameboy.peripherals.serial.recv(0xFF);
      },
    };
    ret
  }

  pub fn emulate(&mut self, cycles: u32, inputs1_js: JsValue, inputs2_js: JsValue) {
    let mut inputs1: Vec<Input> = serde_wasm_bindgen::from_value::<Vec<Input>>(inputs1_js).unwrap();
    let mut inputs2: Vec<Input> = serde_wasm_bindgen::from_value::<Vec<Input>>(inputs2_js).unwrap();
    inputs1.reverse();
    inputs2.reverse();
    let apu_callback = self.gameboy.peripherals.apu.callback.take();
    for i in 0..=cycles {
      if let Some(input) = inputs1.last() {
        if i == input.cycle {
          if input.down {
            self.key_down(&input.code);
          } else {
            self.key_up(&input.code);
          }
          inputs1.pop();
        }        
      }
      if let Some(input) = inputs2.last() {
        if i == input.cycle {
          if input.down {
            self.key_down2(&input.code);
          } else {
            self.key_up2(&input.code);
          }
          inputs2.pop();
        }
      }
      self.emulate_cycle();
    }
    assert!(inputs1.is_empty());
    assert!(inputs2.is_empty());
    self.gameboy.peripherals.apu.set_callback(apu_callback.unwrap());
  }

  pub fn frame_buffer(&self) -> Uint8ClampedArray {
    Uint8ClampedArray::from(self.gameboy.peripherals.ppu.buffer.as_ref())
  }

  pub fn key_down(&mut self, k: &str) -> bool {
    if let Some(j) = key2joy(k) {
      self.gameboy.peripherals.joypad.button_down(&mut self.gameboy.cpu.interrupts, j);
      return true;
    }
    false
  }

  pub fn key_up(&mut self, k: &str) -> bool {
    if let Some(j) = key2joy(k) {
      self.gameboy.peripherals.joypad.button_up(j);
      return true;
    }
    false
  }

  pub fn key_down2(&mut self, k: &str) {
    match self.gameboy2.as_mut() {
      Some(gb) => { key2joy(k).map(|j| gb.peripherals.joypad.button_down(&mut gb.cpu.interrupts, j)); },
      None => {},
    }
  }

  pub fn key_up2(&mut self, k: &str) {
    match self.gameboy2.as_mut() {
      Some(gb) => { key2joy(k).map(|j| gb.peripherals.joypad.button_up(j)); },
      None => {},
    }
  }
}

#[wasm_bindgen]
pub struct AudioHandle(OutputStream, OutputStreamHandle, Sink);

#[wasm_bindgen]
impl AudioHandle {
  pub fn new() -> Self {
    let (stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();
    sink.play();
    Self(stream, handle, sink)
  }
  pub fn append(&self, buffer: &[f32]) {
    self.2.append(SamplesBuffer::new(2, SAMPLE_RATE as u32, buffer));
  }
  pub fn length(&self) -> usize {
    self.2.len()
  }
}