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
      self.vram[addr as usize & 0x1fff]
    }
  }
  pub fn read_oam(&self, addr: u16) -> u8 {
    if self.mode == Mode::Drawing || self.mode == Mode::OamScan {
      0xFF
    } else {
      self.oam[addr as usize & 0xff]
    }
  }
  pub fn write_vram(&mut self, addr: u16, val: u8) {
    if self.mode != Mode::Drawing {
      self.vram[addr as usize & 0x1fff] = val;
    }
  }
  pub fn write_oam(&mut self, addr: u16, val: u8) {
    if self.mode != Mode::Drawing && self.mode != Mode::OamScan {
      self.oam[addr as usize & 0xff] = val;
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
        self.draw();
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
  fn draw(&mut self) {
    if self.lcdc & BG_WINDOW_ENABLE > 0 {
      let map_mask: usize = if self.lcdc & BG_TILE_MAP > 0 {
        0x1C00
      } else {
        0x1800
      };

      let y = self.ly.wrapping_add(self.scy);
      let map_row: usize = (y / 8) as usize;
      for i in 0..LCD_WIDTH {
        let x = (i as u8).wrapping_add(self.scx);
        let map_col = (x / 8) as usize;

        let tile_num = if self.lcdc & TILE_DATA > 0 {
          self.vram[((map_row * 32 + map_col) | map_mask) & 0x1fff] as usize
        } else {
          128 + ((self.vram[((map_row * 32 + map_col) | map_mask) & 0x1fff] as i8 as i16) + 128) as usize
        };

        let tile_mask = tile_num << 4;
        let tile_row = ((y % 8) * 2) as usize;
        let data1 = self.vram[(tile_row | tile_mask) & 0x1fff];
        let data2 = self.vram[((tile_row + 1) | tile_mask) & 0x1fff];
        let tile_col = (7 - x % 8) as usize;
        let color_idx = (((data2 >> tile_col) & 1) << 1) | ((data1 >> tile_col) & 1);
        let color = self.bgp_read_color(color_idx);
        self.pixel_buffer[LCD_WIDTH * self.ly as usize + i] = color;
      }
    }
    if self.lcdc & BG_WINDOW_ENABLE > 0 && self.lcdc & WINDOW_DISPLAY_ENABLE > 0 && self.wy <= self.ly {
      let map_mask: usize = if self.lcdc & WINDOW_TILE_MAP > 0 {
        0x1C00
      } else {
        0x1800
      };
      let wx = self.wx.wrapping_sub(7);

      let y = self.ly.wrapping_sub(self.wy);
      let map_row: usize = (y / 8) as usize;
      for i in (wx as usize)..LCD_WIDTH {
        let x = (i as u8).wrapping_sub(wx);
        let map_col = (x / 8) as usize;

        let tile_num = if self.lcdc & TILE_DATA > 0 {
          self.vram[((map_row * 32 + map_col) | map_mask) & 0x1fff] as usize
        } else {
          128 + ((self.vram[((map_row * 32 + map_col) | map_mask) & 0x1fff] as i8 as i16) + 128) as usize
        };

        let tile_mask = tile_num << 4;
        let tile_row = ((y % 8) * 2) as usize;
        let data1 = self.vram[(tile_row | tile_mask) & 0x1fff];
        let data2 = self.vram[((tile_row + 1) | tile_mask) & 0x1fff];
        let tile_col = (7 - x % 8) as usize;
        let color_idx = (((data2 >> tile_col) & 1) << 1) | ((data1 >> tile_col) & 1);
        let color = self.bgp_read_color(color_idx);
        self.pixel_buffer[LCD_WIDTH * self.ly as usize + i] = color;
      }
    }
    if self.lcdc & SPRITE_ENABLE > 0 {

    }
  }

  fn bgp_read_color(&self, idx: u8) -> Color {
    match idx {
      0 => Color::from_u8(self.bgp & 0b11),
      1 => Color::from_u8((self.bgp >> 2) & 0b11),
      2 => Color::from_u8((self.bgp >> 4) & 0b11),
      _ => Color::from_u8((self.bgp >> 6) & 0b11),
    }
  }
}
