use std::{
  cmp::Ordering,
  iter,
};
use log::warn;

use crate::cpu::interrupts;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

const BG_WINDOW_ENABLE: u8 = 1 << 0;
const SPRITE_ENABLE: u8 = 1 << 1;
const SPRITE_SIZE: u8 = 1 << 2;
const BG_TILE_MAP: u8 = 1 << 3;
const TILE_DATA_ADDRESSING_MODE: u8 = 1 << 4;
const WINDOW_DISPLAY_ENABLE: u8 = 1 << 5;
const WINDOW_TILE_MAP: u8 = 1 << 6;
const LCD_DISPLAY_ENABLE: u8 = 1 << 7;

const LYC_EQ_LY: u8 = 1 << 2;
const HBLANK_INT: u8 = 1 << 3;
const VBLANK_INT: u8 = 1 << 4;
const OAM_SCAN_INT: u8 = 1 << 5;
const LYC_EQ_LY_INT: u8 = 1 << 6;

const PALETTE: u8 = 1 << 4;
const X_FLIP: u8 = 1 << 5;
const Y_FLIP: u8 = 1 << 6;
const OBJ2BG_PRIORITY: u8 = 1 << 7;

#[inline]
fn get_color(col: u8) -> u8 {
  match col {
    0 => 0xFF,
    1 => 0xAA,
    2 => 0x55,
    _ => 0x00,
  }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum Mode {
  HBlank = 0,
  VBlank = 1,
  OamScan = 2,
  Drawing = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Sprite {
  y: u8,
  x: u8,
  tile_idx: u8,
  flags: u8,
}

pub struct Ppu {
  mode: Mode,
  lcdc: u8,
  stat: u8,
  scy: u8,
  scx: u8,
  ly: u8,
  lyc: u8,
  bgp: u8,
  obp0: u8,
  obp1: u8,
  wy: u8,
  wx: u8,
  vram: Box<[u8; 0x2000]>,
  oam: Box<[u8; 0xA0]>,
  pub oam_dma: Option<u16>,
  irq: Box<dyn Fn(u8)>,
  cycles: u8,
  pixel_buffer: Box<[u8; LCD_PIXELS]>,
}

impl Ppu {
  pub fn new(irq: Box<dyn Fn(u8)>) -> Self {
    Self {
      mode: Mode::OamScan,
      lcdc: 0,
      stat: 0,
      scy: 0,
      scx: 0,
      ly: 0,
      lyc: 0,
      bgp: 0,
      obp0: 0,
      obp1: 0,
      wy: 0,
      wx: 0,
      vram: Box::new([0; 0x2000]),
      oam: Box::new([0; 0xA0]),
      oam_dma: None,
      irq,
      cycles: 0,
      pixel_buffer: Box::new([0; LCD_PIXELS]),
    }
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0x8000..=0x9FFF => if self.mode == Mode::Drawing {
        0xFF
      } else {
        self.vram[addr as usize & 0x1FFF]
      },
      0xFE00..=0xFE9F => if self.mode == Mode::Drawing || self.mode == Mode::OamScan {
        0xFF
      } else {
        if self.oam_dma.is_some() {
          0xFF
        } else {
          self.oam[addr as usize & 0xFF]
        }
      },
      0xFF40          => self.lcdc,
      0xFF41          => 0x80 | self.stat | self.mode as u8,
      0xFF42          => self.scy,
      0xFF43          => self.scx,
      0xFF44          => self.ly,
      0xFF45          => self.lyc,
      0xFF46          => 0xFF,
      0xFF47          => self.bgp,
      0xFF48          => self.obp0,
      0xFF49          => self.obp1,
      0xFF4A          => self.wy,
      0xFF4B          => self.wx,
      _               => unreachable!(),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0x8000..=0x9FFF => if self.mode != Mode::Drawing {
        self.vram[addr as usize & 0x1FFF] = val;
      },
      0xFE00..=0xFE9F => if self.mode != Mode::Drawing && self.mode != Mode::OamScan {
        if self.oam_dma.is_none() {
          self.oam[addr as usize & 0xFF] = val;
        }
      },
      0xFF40          => {
        if val & LCD_DISPLAY_ENABLE == 0 && self.lcdc & LCD_DISPLAY_ENABLE > 0 {
          if self.mode != Mode::VBlank {
            warn!("Turning off LCD outside VBlank");
          }
          self.ly = 0;
          self.mode = Mode::HBlank;
        }
        if val & LCD_DISPLAY_ENABLE > 0 && self.lcdc & LCD_DISPLAY_ENABLE == 0 {
          self.mode = Mode::OamScan;
          self.cycles = 20;
        }
        self.lcdc = val;
      },
      0xFF41          => self.stat = (self.stat & LYC_EQ_LY) | (val & 0xF8),
      0xFF42          => self.scy = val,
      0xFF43          => self.scx = val,
      0xFF44          => self.ly = 0,
      0xFF45          => self.lyc = val,
      0xFF46          => {
        assert!(val <= 0xDF);
        self.oam_dma =  Some((val as u16) << 8);
      },
      0xFF47          => self.bgp = val,
      0xFF48          => self.obp0 = val,
      0xFF49          => self.obp1 = val,
      0xFF4A          => self.wy = val,
      0xFF4B          => self.wx = val,
      _               => unreachable!(),
    }
  }
  pub fn emulate_cycle(&mut self) -> bool {
    if self.lcdc & LCD_DISPLAY_ENABLE == 0 {
      return false;
    }

    self.cycles -= 1;
    if self.cycles > 0 {
      return false;
    }

    let mut ret = false;
    match self.mode {
      Mode::HBlank => {
        self.ly += 1;
        if self.ly < 144 {
          self.mode = Mode::OamScan;
          self.cycles = 20;
        } else {
          ret = true;
          self.mode = Mode::VBlank;
          self.cycles = 114;
          (self.irq)(interrupts::VBLANK);
          if self.stat & VBLANK_INT > 0 {
            (self.irq)(interrupts::STAT);
          }
        }
        self.check_lyc_eq_ly();
      },
      Mode::VBlank => {
        self.ly += 1;
        if self.ly > 153 {
          self.ly = 0;
          self.mode = Mode::OamScan;
          self.cycles = 20;
          if self.stat & OAM_SCAN_INT > 0 {
            (self.irq)(interrupts::STAT);
          }
        } else {
          self.cycles = 114;
        }
        self.check_lyc_eq_ly();
      },
      Mode::OamScan => {
        self.mode = Mode::Drawing;
        self.cycles = 43;
      },
      Mode::Drawing => {
        self.render();
        self.mode = Mode::HBlank;
        self.cycles = 51;
        if self.stat & HBLANK_INT > 0 {
          (self.irq)(interrupts::STAT);
        }
      },
    }
    ret
  }
  pub fn oam_dma_emulate_cycle(&mut self, val: u8) {
    if let Some(addr) = self.oam_dma {
      if self.mode != Mode::Drawing && self.mode != Mode::OamScan {
        self.oam[addr as usize & 0xFF] = val;
      }
      self.oam_dma = Some(addr.wrapping_add(1)).filter(|&x| (x as u8) < 0xA0);
    }
  }
  pub fn pixel_buffer(&self) -> Box<[u8]> {
    self.pixel_buffer.iter().flat_map(
      |&e| iter::repeat(e.into()).take(3)
    ).collect::<Box<[u8]>>()
  }
  fn render(&mut self) {
    let mut bg_prio: [bool; 160] = [false; LCD_WIDTH];
    if self.lcdc & BG_WINDOW_ENABLE > 0 {
      self.render_bg_window(false, &mut bg_prio);
    }
    if self.lcdc & BG_WINDOW_ENABLE > 0 && self.lcdc & WINDOW_DISPLAY_ENABLE > 0 && self.wy <= self.ly {
      self.render_bg_window(true, &mut bg_prio);
    }
    if self.lcdc & SPRITE_ENABLE > 0 {
      let size = if self.lcdc & SPRITE_SIZE > 0 { 16 } else { 8 };

      let mut sprites: Vec<(usize, Sprite)> = unsafe {
        std::mem::transmute::<[u8; 0xA0], [Sprite; 40]>(
          self.oam.as_ref().clone()
        )
      }.into_iter().filter_map(|mut sprite| {
        sprite.y = sprite.y.wrapping_sub(16);
        sprite.x = sprite.x.wrapping_sub(8);
        if self.ly.wrapping_sub(sprite.y) < size {
          Some(sprite)
        } else {
          None
        }
      }).take(10).enumerate().collect();
      sprites.sort_by(|&(a_idx, a), &(b_idx, b)| {
        match b.x.cmp(&a.x) {
          Ordering::Equal => b_idx.cmp(&a_idx),
          other => other,
        }
      });

      for (_, sprite) in sprites {
        let palette = if sprite.flags & PALETTE > 0 { self.obp1 } else { self.obp0 };
        let mut tile_idx = sprite.tile_idx as usize;
        let mut tile_row = if sprite.flags & Y_FLIP > 0 {
          size - 1 - self.ly.wrapping_sub(sprite.y)
        } else {
          self.ly.wrapping_sub(sprite.y)
        };

        // if the size is 16 and it is second tile
        assert!(tile_row < 16);
        tile_idx += (tile_row >= 8) as usize;
        tile_row %= 8;

        for tile_col in 0..8 {
          let tile_col_flipped = if sprite.flags & X_FLIP > 0 {
            7 - tile_col
          } else {
            tile_col
          };
          let palette_idx = self.get_palette_idx_from_tile(tile_idx, tile_row, tile_col_flipped);
          let color = get_color((palette >> (palette_idx << 1)) & 0b11);
          let i = sprite.x.wrapping_add(tile_col) as usize;
          if i < LCD_WIDTH && palette_idx > 0 {
            if sprite.flags & OBJ2BG_PRIORITY == 0 || !bg_prio[i] {
              self.pixel_buffer[LCD_WIDTH * self.ly as usize + i] = color;
            }
          }
        }
      }
    }
  }
  fn render_bg_window(&mut self, bg_window: bool, bg_prio: &mut [bool; LCD_WIDTH]) {
    let wx = if bg_window { self.wx.wrapping_sub(7) } else { 0 };
    let scx = if bg_window { 0 } else { self.scx };
    let y = if bg_window { self.ly.wrapping_sub(self.wy) } else { self.ly.wrapping_add(self.scy) };
    for i in (wx as usize)..LCD_WIDTH {
      let x = (i as u8).wrapping_sub(wx).wrapping_add(scx);
      let tile_idx = self.get_tile_num_from_tile_map(
        (self.lcdc & if bg_window { WINDOW_TILE_MAP } else { BG_TILE_MAP }) > 0,
        y / 8, x / 8
      );
      let palette_idx = self.get_palette_idx_from_tile(tile_idx, y % 8, x % 8);
      let color = get_color((self.bgp >> (palette_idx << 1)) & 0b11);
      self.pixel_buffer[LCD_WIDTH * self.ly as usize + i] = color;
      bg_prio[i] = palette_idx != 0;
    }
  }
  fn get_tile_num_from_tile_map(&self, tile_map: bool, map_row: u8, map_col: u8) -> usize {
    let start: usize = 0x1800 | ((tile_map as usize) << 10);
    let ret = self.vram[start | ((((map_row as usize) << 5) + map_col as usize) & 0x3FF)];
    if self.lcdc & TILE_DATA_ADDRESSING_MODE > 0 {
      ret as usize
    } else {
      ((ret as i8 as i16) + 0x100) as usize
    }
  }
  fn get_palette_idx_from_tile(&self, tile_idx: usize, tile_row: u8, tile_col: u8) -> u8{
    let row = (tile_row * 2) as usize;
    let col = (7 - tile_col) as usize;
    let start = tile_idx << 4;
    let low = self.vram[(start | row) & 0x1FFF];
    let high = self.vram[(start | (row + 1)) & 0x1FFF];
    (((high >> col) & 1) << 1) | ((low >> col) & 1)
  }
  fn check_lyc_eq_ly(&mut self) {
    if self.ly == self.lyc {
      self.stat |= LYC_EQ_LY;
      if self.stat & LYC_EQ_LY_INT > 0 {
        (self.irq)(interrupts::STAT);
      }
    } else {
      self.stat &= !LYC_EQ_LY;
    }
  }
}
