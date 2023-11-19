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
    let bootrom = Bootrom::new(vec![
      0x31, 0xfe, 0xff, 0x21, 0x00, 0x80, 0x3e, 0x00, 0x22, 0xcb, 0x6c, 0x28, 0xf9, 0x3e, 0x80, 0xe0,
      0x26, 0xe0, 0x11, 0x3e, 0xf3, 0xe0, 0x12, 0xe0, 0x25, 0x3e, 0x77, 0xe0, 0x24, 0x3e, 0xfc, 0xe0,
      0x47, 0x11, 0xaa, 0x00, 0x21, 0x10, 0x80, 0x1a, 0x47, 0xcd, 0x7a, 0x00, 0xcd, 0x7a, 0x00, 0x13,
      0x7b, 0xfe, 0x34, 0x20, 0xf2, 0x3e, 0x19, 0xea, 0x10, 0x99, 0x21, 0x2f, 0x99, 0x0e, 0x0c, 0x3d,
      0x28, 0x08, 0x32, 0x0d, 0x20, 0xf9, 0x2e, 0x0f, 0x18, 0xf5, 0x3e, 0x64, 0xe0, 0x43, 0x57, 0x3e,
      0x91, 0xe0, 0x40, 0xcd, 0x8f, 0x00, 0xcd, 0x8f, 0x00, 0x15, 0x7a, 0xe0, 0x43, 0x20, 0xf4, 0x3e,
      0x83, 0xcd, 0xa3, 0x00, 0x06, 0xa0, 0xcd, 0x9c, 0x00, 0x21, 0xb0, 0x01, 0xe5, 0xf1, 0x21, 0x4d,
      0x01, 0x01, 0x13, 0x00, 0x11, 0xd8, 0x00, 0xc3, 0xfe, 0x00, 0x3e, 0x04, 0x0e, 0x00, 0xcb, 0x10,
      0xf5, 0xcb, 0x11, 0xf1, 0xcb, 0x11, 0x3d, 0x20, 0xf5, 0x79, 0x22, 0x23, 0x22, 0x23, 0xc9, 0xc5,
      0x06, 0x0a, 0x0e, 0xff, 0x0d, 0x20, 0xfd, 0x05, 0x20, 0xf8, 0xc1, 0xc9, 0xcd, 0x8f, 0x00, 0x05,
      0x20, 0xfa, 0xc9, 0xe0, 0x13, 0x3e, 0x87, 0xe0, 0x14, 0xc9, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00,
      0xff, 0xff, 0xff, 0x33, 0x00, 0x00, 0x33, 0x33, 0xff, 0xcc, 0xcc, 0x00, 0x33, 0x33, 0xff, 0xcc,
      0xcc, 0x00, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0x00, 0xcc, 0xcc, 0x33, 0x33,
      0xff, 0xff, 0x00, 0xcc, 0x33, 0x33, 0xff, 0xff, 0x00, 0xcc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x21, 0x04, 0x01, 0x11, 0xa8, 0x00, 0x1a, 0x13, 0xbe, 0x20, 0x01, 0x23, 0x7d, 0xfe,
      0x34, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x50
    ].into());
    let cartridge = Cartridge::new(cart_rom.into(), if save.len() > 0 {
      Some(save.to_vec())
    } else {
      None
    });
    let peripherals = Peripherals::new(bootrom, cartridge, false);
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
    let mut pixel_buffer = Vec::new();
    for e in self.peripherals.ppu.pixel_buffer().iter() {
      pixel_buffer.push(*e);
      pixel_buffer.push(*e);
      pixel_buffer.push(*e);
      pixel_buffer.push(0xFF);
    }
    Uint8ClampedArray::from(pixel_buffer.as_ref())
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