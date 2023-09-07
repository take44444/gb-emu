use std::{
  cell::RefCell,
  rc::Rc,
};

use crate::{
  bootrom::Bootrom,
  cartridge::Cartridge,
  ppu::Ppu,
  apu::Apu,
  hram::HRam,
  wram::WRam,
  cpu::interrupts::Interrupts,
  timer::Timer,
  joypad::Joypad,
  audio::Audio,
};

pub struct Peripherals {
  pub bootrom: Bootrom,
  pub cartridge: Cartridge,
  pub ppu: Ppu,
  pub apu: Apu,
  pub timer: Timer,
  pub joypad: Joypad,
  pub hram: HRam,
  pub wram: WRam,
  interrupts: Rc<RefCell<Interrupts>>,
}

impl Peripherals {
  pub fn new(bootrom: Bootrom, cartridge: Cartridge, audio: Audio, interrupts: Rc<RefCell<Interrupts>>) -> Self {
    let i1 = interrupts.clone();
    let i2 = interrupts.clone();
    let i3 = interrupts.clone();
    Self {
      bootrom,
      cartridge,
      ppu: Ppu::new(Box::new(move |val| i1.borrow_mut().irq(val))),
      apu: Apu::new(audio),
      timer: Timer::new(Box::new(move |val| i2.borrow_mut().irq(val))),
      joypad: Joypad::new(Box::new(move |val| i3.borrow_mut().irq(val))),
      hram: HRam::new(),
      wram: WRam::new(),
      interrupts,
    }
  }
  pub fn emulate_cycle(&mut self) -> bool {
    self.timer.emulate_cycle();
    self.apu.emulate_cycle();
    if let Some(addr) = self.ppu.oam_dma {
      self.ppu.oam_dma_emulate_cycle(self.read(addr));
    }
    self.ppu.emulate_cycle()
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0x0000..=0x00FF => if self.bootrom.is_active() {
        self.bootrom.read(addr)
      } else {
        self.cartridge.read(addr)
      },
      0x0100..=0x7FFF => self.cartridge.read(addr),
      0x8000..=0x9FFF => self.ppu.read(addr),
      0xA000..=0xBFFF => self.cartridge.read(addr),
      0xC000..=0xDFFF => self.wram.read(addr),
      0xE000..=0xFDFF => self.wram.read(addr),
      0xFE00..=0xFE9F => self.ppu.read(addr),
      0xFF00          => self.joypad.read(),
      0xFF04..=0xFF07 => self.timer.read(addr),
      0xFF0F          => self.interrupts.borrow().read(addr),
      0xFF10..=0xFF26 | 0xFF30..=0xFF3F => self.apu.read(addr),
      0xFF40..=0xFF4B => self.ppu.read(addr),
      0xFF80..=0xFFFE => self.hram.read(addr),
      0xFFFF          => self.interrupts.borrow().read(addr),
      _               => 0xFF,
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0x0000..=0x00FF => if !self.bootrom.is_active() {
        self.cartridge.write(addr, val)
      }
      0x0100..=0x7FFF => self.cartridge.write(addr, val),
      0x8000..=0x9FFF => self.ppu.write(addr, val),
      0xA000..=0xBFFF => self.cartridge.write(addr, val),
      0xC000..=0xDFFF => self.wram.write(addr, val),
      0xE000..=0xFDFF => self.wram.write(addr, val),
      0xFE00..=0xFE9F => self.ppu.write(addr, val),
      0xFF00          => self.joypad.write(val),
      0xFF04..=0xFF07 => self.timer.write(addr, val),
      0xFF0F          => self.interrupts.borrow_mut().write(addr, val),
      0xFF10..=0xFF26 | 0xFF30..=0xFF3F => self.apu.write(addr, val),
      0xFF40..=0xFF4B => self.ppu.write(addr, val),
      0xFF50          => self.bootrom.write(addr, val),
      0xFF80..=0xFFFE => self.hram.write(addr, val),
      0xFFFF          => self.interrupts.borrow_mut().write(addr, val),
      _               => (),
    }
  }
}