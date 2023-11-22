use std::rc::Rc;

use js_sys::{Float32Array, Function, Uint8ClampedArray, Uint8Array, JsString};
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
use wasm_bindgen::prelude::*;

use gbemu::{
  bootrom::Bootrom,
  cartridge::Cartridge,
  cpu::Cpu,
  peripherals::Peripherals,
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
pub struct GameBoyHandle{
  cpu: Cpu,
  peripherals: Peripherals,
  // snapshots: VecDeque<(Cpu, Peripherals)>,
  // cnt: usize,
}

#[wasm_bindgen]
impl GameBoyHandle {
  pub fn new(cart_rom: &[u8], save: &[u8]) -> Self {
    console_error_panic_hook::set_once();
    let cartridge = Cartridge::new(cart_rom.into(), if save.len() > 0 {
      Some(save.to_vec())
    } else {
      None
    });
    let is_cgb = cartridge.is_cgb;
    let bootrom = Bootrom::new();
    let peripherals = Peripherals::new(bootrom, cartridge, is_cgb);
    let cpu = Cpu::new();
    Self {
      cpu,
      peripherals,
      // snapshots: VecDeque::new(),
      // cnt: 0,
    }
  }

  pub fn set_callback(&mut self, apu_callback: Function, send_callback: Function) {
    let apu_callback = Rc::new(move |buffer: &[f32]| {
      apu_callback
        .call1(&JsValue::null(), &Float32Array::from(buffer))
        .unwrap();
    });
    let send_callback = Rc::new(move |val: u8/*, rollback: usize*/| {
      send_callback
        .call1(&JsValue::null(), &JsValue::from(val)/*, &JsValue::from(rollback)*/)
        .unwrap();
    });
    self.peripherals.apu.set_callback(apu_callback);
    self.peripherals.serial.set_callback(send_callback);
  }

  pub fn title(&self) -> JsString {
    JsString::from(self.peripherals.cartridge.title.clone())
  }

  pub fn save(&self) -> Uint8Array {
    Uint8Array::from(self.peripherals.cartridge.sram.as_ref())
  }

  // pub fn emulate_frame(&mut self) -> Uint8ClampedArray {
  //   loop {
  //     if self.emulate_cycle() {
  //       let mut pixel_buffer = Vec::new();
  //       for e in self.peripherals.ppu.pixel_buffer().iter() {
  //         pixel_buffer.push(*e);
  //         pixel_buffer.push(*e);
  //         pixel_buffer.push(*e);
  //         pixel_buffer.push(0xFF);
  //       }
  //       return Uint8ClampedArray::from(pixel_buffer.as_ref());
  //     }
  //   }
  // }

  // pub fn rollback(&mut self, idx: usize) -> usize {
  //   let mut ret = 0;
  //   if self.snapshots.len() > idx {
  //     ret = self.snapshots.len() - idx;
  //     self.cpu = self.snapshots[idx].0.clone();
  //     self.peripherals = self.snapshots[idx].1.clone();
  //   }
  //   self.cnt = 0;
  //   self.snapshots.clear();
  //   ret
  // }

  pub fn emulate_cycle(&mut self) -> bool {
    // if self.cnt > 200 {
    //   self.cnt = 0;
    //   self.snapshots.push_back((self.cpu.clone(), self.peripherals.clone()));
    //   if self.snapshots.len() > 1000 {
    //     self.snapshots.pop_front();
    //   }
    // }
    // self.cnt += 1;
    self.cpu.emulate_cycle(&mut self.peripherals);
    self.peripherals.timer.emulate_cycle(&mut self.cpu.interrupts);
    self.peripherals.serial.emulate_cycle(&mut self.cpu.interrupts);
    self.peripherals.apu.emulate_cycle();
    if let Some(addr) = self.peripherals.ppu.oam_dma {
      self.peripherals.ppu.oam_dma_emulate_cycle(self.peripherals.read(&self.cpu.interrupts, addr));
    }
    self.peripherals.ppu.emulate_cycle(&mut self.cpu.interrupts)
  }

  pub fn frame_buffer(&self) -> Uint8ClampedArray {
    Uint8ClampedArray::from(self.peripherals.ppu.pixel_buffer().as_ref())
  }

  pub fn key_down(&mut self, k: &str) {
    key2joy(k).map(|j| self.peripherals.joypad.button_down(&mut self.cpu.interrupts, j));
  }

  pub fn key_up(&mut self, k: &str) {
    key2joy(k).map(|j| self.peripherals.joypad.button_up(j));
  }

  pub fn serial_is_master(&self) -> JsValue {
    JsValue::from(self.peripherals.serial.is_master())
  }

  pub fn serial_receive(&mut self, val: u8) {
    self.peripherals.serial.receive(val);
  }

  pub fn serial_data(&self) -> u8 {
    self.peripherals.serial.data
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