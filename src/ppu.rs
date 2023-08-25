use std::cmp::Ordering;

use crate::interrupts;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

pub const BG_WINDOW_ENABLE: u8 = 1 << 0;
pub const SPRITE_ENABLE: u8 = 1 << 1;
pub const SPRITE_SIZE: u8 = 1 << 2;
pub const BG_TILE_MAP: u8 = 1 << 3;
pub const TILE_DATA: u8 = 1 << 4;
pub const WINDOW_DISPLAY_ENABLE: u8 = 1 << 5;
pub const WINDOW_TILE_MAP: u8 = 1 << 6;
pub const LCD_DISPLAY_ENABLE: u8 = 1 << 7;

pub const LYC_EQ_LY: u8 = 1 << 2;
pub const HBLANK_INT: u8 = 1 << 3;
pub const VBLANK_INT: u8 = 1 << 4;
pub const OAM_SCAN_INT: u8 = 1 << 5;
pub const LYC_EQ_LY_INT: u8 = 1 << 6;

pub const PALETTE: u8 = 1 << 4;
pub const X_FLIP: u8 = 1 << 5;
pub const Y_FLIP: u8 = 1 << 6;
pub const OBJ2BG_PRIORITY: u8 = 1 << 7;

fn pallete_read_color(idx: u8, palette: u8) -> Color {
  match idx {
    0 => Color::from_u8(palette & 0b11),
    1 => Color::from_u8((palette >> 2) & 0b11),
    2 => Color::from_u8((palette >> 4) & 0b11),
    _ => Color::from_u8((palette >> 6) & 0b11),
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Color {
  White,
  LightGray,
  DarkGray,
  Black,
}

impl Color {
  #[inline]
  pub fn from_u8(val: u8) -> Color {
    match val {
      1 => Self::LightGray,
      2 => Self::DarkGray,
      3 => Self::Black,
      _ => Self::White,
    }
  }
}

impl Into<u8> for Color {
  fn into(self) -> u8 {
    match self {
      Self::White => 0xFF,
      Self::LightGray => 0xAA,
      Self::DarkGray => 0x55,
      Self::Black => 0x00,
    }
  }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Sprite {
  y: u8,
  x: u8,
  tile_num: u8,
  flags: u8,
}

pub struct Ppu {
  pub pixel_buffer: Box<[Color; LCD_PIXELS]>,
  mode: Mode,
  lcdc: u8,
  stat: u8,
  ly: u8,
  lyc: u8,
  scx: u8,
  scy: u8,
  wx: u8,
  wy: u8,
  bgp: u8,
  obp0: u8,
  obp1: u8,
  vram: Box<[u8; 0x2000]>,
  oam: Box<[u8; 0x100]>,
  cycles: isize,
}

impl Ppu {
  pub fn new() -> Self {
    Self {
      pixel_buffer: Box::new([Color::White; LCD_PIXELS]),
      mode: Mode::OamScan,
      lcdc: 0,
      stat: 0,
      ly: 0,
      lyc: 0,
      scx: 0,
      scy: 0,
      wx: 0,
      wy: 0,
      bgp: 0,
      obp0: 0,
      obp1: 0,
      vram: Box::new([0; 0x2000]),
      oam: Box::new([0; 0x100]),
      cycles: 0,
    }
  }
  pub fn read_lcdc(&self) -> u8 {
    self.lcdc
  }
  pub fn read_stat(&self) -> u8 {
    if self.lcdc & LCD_DISPLAY_ENABLE == 0 {
      0x80
    } else {
      self.mode as u8 | self.stat | 0x80
    }
  }
  pub fn read_ly(&self) -> u8 {
    self.ly
  }
  pub fn read_lyc(&self) -> u8 {
    self.lyc
  }
  pub fn read_scx(&self) -> u8 {
    self.scx
  }
  pub fn read_scy(&self) -> u8 {
    self.scy
  }
  pub fn read_wx(&self) -> u8 {
    self.wx
  }
  pub fn read_wy(&self) -> u8 {
    self.wy
  }
  pub fn read_bgp(&self) -> u8 {
    self.bgp
  }
  pub fn read_obp0(&self) -> u8 {
    self.obp0
  }
  pub fn read_obp1(&self) -> u8 {
    self.obp1
  }
  pub fn write_lcdc(&mut self, val: u8) {
    if val & LCD_DISPLAY_ENABLE == 0 && self.lcdc & LCD_DISPLAY_ENABLE > 0 {
      if self.mode != Mode::VBlank {
        panic!("Warning! LCD off, but not in VBlank");
      }
      self.ly = 0;
    }
    if val & LCD_DISPLAY_ENABLE > 0 && self.lcdc & LCD_DISPLAY_ENABLE == 0 {
      self.mode = Mode::HBlank;
      self.cycles = 21;
      self.stat |= LYC_EQ_LY;
    }
    self.lcdc = val;
  }
  pub fn write_stat(&mut self, val: u8) {
    self.stat = (self.stat & LYC_EQ_LY)
      | (val & HBLANK_INT)
      | (val & VBLANK_INT)
      | (val & OAM_SCAN_INT)
      | (val & LYC_EQ_LY_INT);
  }
  pub fn reset_ly(&mut self) {
    self.ly = 0;
  }
  pub fn write_lyc(&mut self, val: u8) {
    self.lyc = val;
  }
  pub fn write_scx(&mut self, val: u8) {
    self.scx = val;
  }
  pub fn write_scy(&mut self, val: u8) {
    self.scy = val;
  }
  pub fn write_wx(&mut self, val: u8) {
    self.wx = val;
  }
  pub fn write_wy(&mut self, val: u8) {
    self.wy = val;
  }
  pub fn write_bgp(&mut self, val: u8) {
    self.bgp = val;
  }
  pub fn write_obp0(&mut self, val: u8) {
    self.obp0 = val;
  }
  pub fn write_obp1(&mut self, val: u8) {
    self.obp1 = val;
  }
  pub fn read_vram(&self, addr: u16) -> u8 {
    if self.mode == Mode::Drawing {
      0xFF
    } else {
      self.vram[addr as usize & 0x1FFF]
    }
  }
  pub fn read_oam(&self, addr: u16) -> u8 {
    if self.mode == Mode::Drawing || self.mode == Mode::OamScan {
      0xFF
    } else {
      self.oam[addr as usize & 0xFF]
    }
  }
  pub fn write_vram(&mut self, addr: u16, val: u8) {
    if self.mode != Mode::Drawing {
      self.vram[addr as usize & 0x1FFF] = val;
    }
  }
  pub fn write_oam(&mut self, addr: u16, val: u8) {
    if self.mode != Mode::Drawing && self.mode != Mode::OamScan {
      self.oam[addr as usize & 0xFF] = val;
    }
  }
  fn change_mode(&mut self, interrupts: &mut interrupts::Interrupts, mode: Mode) {
    self.mode = mode;
    let adjust = match self.scx % 8 {
      5..=7 => 2,
      1..=4 => 1,
      _ => 0,
    };
    match self.mode {
      Mode::HBlank => {
        self.cycles += 50 - adjust;
      },
      Mode::VBlank => {
        self.cycles += 114;
        interrupts.write_if(interrupts.read_if() | interrupts::VBLANK);
        if self.stat & VBLANK_INT > 0 {
          interrupts.write_if(interrupts.read_if() | interrupts::STAT);
        }
        if self.stat & OAM_SCAN_INT > 0 {
          interrupts.write_if(interrupts.read_if() | interrupts::STAT);
        }
      },
      Mode::OamScan => {
        self.cycles += 21;
        if self.stat & OAM_SCAN_INT > 0 {
          interrupts.write_if(interrupts.read_if() | interrupts::STAT);
        }
      },
      Mode::Drawing => {
        self.cycles += 43 + adjust;
      },
    }
  }
  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) -> bool {
    if self.lcdc & LCD_DISPLAY_ENABLE == 0 {
      return false;
    }

    self.cycles -= 1;
    if self.cycles == 1 && self.mode == Mode::Drawing {
      if self.stat & HBLANK_INT > 0 {
        interrupts.write_if(interrupts.read_if() | interrupts::STAT);
      }
    }
    if self.cycles > 0 {
      return false;
    }

    let mut ret = false;
    match self.mode {
      Mode::HBlank => {
        self.ly += 1;
        if self.ly < 144 {
          self.change_mode(interrupts, Mode::OamScan);
        } else {
          ret = true;
          self.change_mode(interrupts, Mode::VBlank);
        }
        self.check_lyc_eq_ly(interrupts);
      },
      Mode::VBlank => {
        self.ly += 1;
        if self.ly > 153 {
          self.ly = 0;
          self.change_mode(interrupts, Mode::OamScan);
        } else {
          self.cycles += 114;
        }
        self.check_lyc_eq_ly(interrupts);
      },
      Mode::OamScan => self.change_mode(interrupts, Mode::Drawing),
      Mode::Drawing => {
        self.render();
        self.change_mode(interrupts, Mode::HBlank);
      },
    }
    return ret;
  }
  fn check_lyc_eq_ly(&mut self, interrupts: &mut interrupts::Interrupts) {
    if self.ly != self.lyc {
      self.stat &= !LYC_EQ_LY;
    } else {
      self.stat |= LYC_EQ_LY;
      if self.stat & LYC_EQ_LY_INT > 0 {
        interrupts.write_if(interrupts.read_if() | interrupts::STAT);
      }
    }
  }
  fn render(&mut self) {
    let mut bg_prio = [false; LCD_WIDTH];
    if self.lcdc & BG_WINDOW_ENABLE > 0 {
      self.draw_bg_window(false, &mut bg_prio);
    }
    if self.lcdc & BG_WINDOW_ENABLE > 0 && self.lcdc & WINDOW_DISPLAY_ENABLE > 0 && self.wy <= self.ly {
      self.draw_bg_window(true, &mut bg_prio);
    }
    if self.lcdc & SPRITE_ENABLE > 0 {
      let size = if self.lcdc & SPRITE_SIZE > 0 { 16 } else { 8 };

      let mut sprites: Vec<(usize, Sprite)> = unsafe {
        std::mem::transmute::<[u8; 0x100], [Sprite; 0x40]>(
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
        let mut tile_num = sprite.tile_num as usize;
        let mut tile_row = if sprite.flags & Y_FLIP > 0 {
          size - 1 - self.ly.wrapping_sub(sprite.y)
        } else {
          self.ly.wrapping_sub(sprite.y)
        };

        // if the size is 16 and it is second tile
        assert!(tile_row < 16);
        tile_num += (tile_row >= 8) as usize;
        tile_row %= 8;

        for tile_col in 0..8 {
          let tile_col_flipped = if sprite.flags & X_FLIP > 0 {
            7 - tile_col
          } else {
            tile_col
          };
          let color_idx = self.get_color_from_tile(tile_num, tile_row, tile_col_flipped);
          let color = pallete_read_color(color_idx, palette);
          let i = sprite.x.wrapping_add(tile_col) as usize;
          if i < LCD_WIDTH && color_idx > 0 {
            if sprite.flags & OBJ2BG_PRIORITY == 0 || !bg_prio[i] {
              self.pixel_buffer[LCD_WIDTH * self.ly as usize + i] = color;
            }
          }
        }
      }
    }
  }
  fn draw_bg_window(&mut self, bg_window: bool, bg_prio: &mut [bool; LCD_WIDTH]) {
    let wx = if bg_window { self.wx.wrapping_sub(7) } else { 0 };
    let scx = if bg_window { 0 } else { self.scx };
    let y = if bg_window { self.ly.wrapping_sub(self.wy) } else { self.ly.wrapping_add(self.scy) };
    for i in (wx as usize)..LCD_WIDTH {
      let x = (i as u8).wrapping_sub(wx).wrapping_add(scx);
      let tile_num = self.get_tile_num_from_tile_map(
        (self.lcdc & if bg_window { WINDOW_TILE_MAP } else { BG_TILE_MAP }) > 0,
        y / 8, x / 8
      );
      let color_idx = self.get_color_from_tile(tile_num, y % 8, x % 8);
      let color = pallete_read_color(color_idx, self.bgp);
      self.pixel_buffer[LCD_WIDTH * self.ly as usize + i] = color;
      bg_prio[i] = color_idx != 0;
    }
  }
  fn get_tile_num_from_tile_map(&self, tile_map: bool, map_row: u8, map_col: u8) -> usize {
    let map_mask: usize = if tile_map { 0x1C00 } else { 0x1800 };
    let ret = self.vram[((((map_row as usize) << 5) + map_col as usize) & 0x3FF) | map_mask];
    if self.lcdc & TILE_DATA > 0 {
      ret as usize
    } else {
      ((ret as i8 as i16 as u16) + 256) as usize
    }
  }
  fn get_color_from_tile(&self, tile_num: usize, tile_row: u8, tile_col: u8) -> u8{
    let row = (tile_row * 2) as usize;
    let col = (7 - tile_col) as usize;
    let mask = tile_num << 4;
    let data1 = self.vram[(row | mask) & 0x1FFF];
    let data2 = self.vram[((row + 1) | mask) & 0x1FFF];
    (((data2 >> col) & 1) << 1) | ((data1 >> col) & 1)
  }
}
