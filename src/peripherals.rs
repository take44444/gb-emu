use crate::bootrom;
use crate::cartridge;
use crate::interrupts;
use crate::joypad;
use crate::oam_dma;
use crate::timer;
use crate::wram;
use crate::hram;
use crate::ppu;

pub struct Peripherals {
  wram: wram::WRam,
  hram: hram::HRam,
  pub ppu: ppu::Ppu,
  timer: timer::Timer,
  oam_dma: oam_dma::OamDma,
  pub joypad: joypad::Joypad,
  bootrom: bootrom::Bootrom,
  pub cartridge: cartridge::Cartridge,
  // apu: apu::Apu,
}

impl Peripherals {
  pub fn new(bootrom: bootrom::Bootrom, cartridge: cartridge::Cartridge) -> Self {
    Self {
      wram: wram::WRam::new(),
      hram: hram::HRam::new(),
      ppu: ppu::Ppu::new(),
      timer: timer::Timer::new(),
      oam_dma: oam_dma::OamDma::new(),
      joypad: joypad::Joypad::new(),
      bootrom,
      cartridge,
    }
  }

  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) -> bool {
    self.emulate_oam_dma_cycle(interrupts);
    let ret = self.ppu.emulate_cycle(interrupts);
    self.timer.emulate_cycle(interrupts);
    // self.apu.emulate_cycle();
    ret
  }

  pub fn emulate_oam_dma_cycle(&mut self, interrupts: &mut interrupts::Interrupts) {
    if let Some(addr) = self.oam_dma.addr() {
      let val = if addr >> 8 <= 0xFD {
        self.read(interrupts, addr)
      } else {
        self.wram.read(addr)
      };
      self.ppu.write_oam(addr, val);
    }
    self.oam_dma.start_if_requested();
  }

  pub fn read(&self, interrupts: &interrupts::Interrupts, addr: u16) -> u8 {
    match (addr >> 8) as u8 {
      0x00 if self.bootrom.is_active => self.bootrom.read(addr),
      0x00..=0x3F => self.cartridge.read_0000_3fff(addr),
      0x40..=0x7F => self.cartridge.read_4000_7fff(addr),
      0x80..=0x9F => self.ppu.read_vram(addr),
      0xA0..=0xbF => self.cartridge.read_a000_bfff(addr),
      0xC0..=0xDF => self.wram.read(addr),
      0xE0..=0xFD => self.wram.read(addr),
      0xFE => {
        match addr as u8 {
          0x00..=0x9F => {
            if self.oam_dma.is_running() {
              0xFF
            } else {
              self.ppu.read_oam(addr)
            }
          },
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      },
      0xFF => {
        match addr as u8 {
          0x00 => self.joypad.read(),
          0x04 => self.timer.read_div(),
          0x05 => self.timer.read_tima(),
          0x06 => self.timer.read_tma(),
          0x07 => self.timer.read_tac(),
          0x0F => interrupts.read_if(),
          0x40 => self.ppu.read_lcdc(),
          0x41 => self.ppu.read_stat(),
          0x42 => self.ppu.read_scy(),
          0x43 => self.ppu.read_scx(),
          0x44 => self.ppu.read_ly(),
          0x45 => self.ppu.read_lyc(),
          0x47 => self.ppu.read_bgp(),
          0x48 => self.ppu.read_obp0(),
          0x49 => self.ppu.read_obp1(),
          0x4A => self.ppu.read_wy(),
          0x4B => self.ppu.read_wx(),
          0x80..=0xFE => self.hram.read(addr),
          0xFF => interrupts.read_ie(),
          _ => 0xFF, // panic!("Unsupported read at ${:04x}", addr),
        }
      },
    }
  }

  pub fn write(&mut self, interrupts: &mut interrupts::Interrupts, addr: u16, val: u8) {
    match (addr >> 8) as u8 {
      0x00 if self.bootrom.is_active => (),
      0x00..=0x7F => self.cartridge.write(addr, val),
      0x80..=0x9F => self.ppu.write_vram(addr, val),
      0xA0..=0xBF => self.cartridge.write_a000_bfff(addr, val),
      0xC0..=0xDF => self.wram.write(addr, val),
      0xE0..=0xFD => self.wram.write(addr, val),
      0xFE => {
        match addr as u8 {
          0x00..=0x9F => {
            if !self.oam_dma.is_running() {
              self.ppu.write_oam(addr, val);
            }
          },
          _ => panic!("Unsupported read at ${:04x}", addr),
        }
      },
      0xFF => {
        match addr as u8 {
          0x00 => self.joypad.write(val),
          0x04 => self.timer.reset_div(),
          0x05 => self.timer.write_tima(val),
          0x06 => self.timer.write_tma(val),
          0x07 => self.timer.write_tac(val),
          0x0F => interrupts.write_if(val),
          0x40 => self.ppu.write_lcdc(val),
          0x41 => self.ppu.write_stat(val),
          0x42 => self.ppu.write_scy(val),
          0x43 => self.ppu.write_scx(val),
          0x44 => self.ppu.reset_ly(),
          0x45 => self.ppu.write_lyc(val),
          0x46 => self.oam_dma.request(val),
          0x47 => self.ppu.write_bgp(val),
          0x48 => self.ppu.write_obp0(val),
          0x49 => self.ppu.write_obp1(val),
          0x4A => self.ppu.write_wy(val),
          0x4B => self.ppu.write_wx(val),
          0x50 => {
            if self.bootrom.is_active && val > 0 {
              self.bootrom.is_active = false;
            }
          },
          0x80..=0xFE => self.hram.write(addr, val),
          0xFF => interrupts.write_ie(val),
          _ => () // panic!("Unsupported read at ${:04x}", addr),
        }
      }
    }
  }
}