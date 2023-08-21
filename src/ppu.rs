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
#[repr(u8)]
pub enum Color {
  Off = 0,
  Light = 1,
  Dark = 2,
  On = 3,
}

pub struct Ppu {
  mode: Mode,
  lcdc: u8,
  stat: u8,
  ly: u8,
  lyc: u8,
  scx: u8,
  scy: u8,
  wx: u8,
  wy: u8,
  vram: Box<[u8; 0x2000]>,
  oam: Box<[u8; 0x100]>,
}

impl Ppu {
  pub fn new() -> Self {
    Self {
      mode: Mode::OamScan,
      lcdc: 0,
      stat: 0,
      ly: 0,
      lyc: 0,
      scx: 0,
      scy: 0,
      wx: 0,
      wy: 0,
      vram: Box::new([0; 0x2000]),
      oam: Box::new([0; 0x100]),
    }
  }

  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) {
    if !self.lcdc & LCD_DISPLAY_ENABLE > 0 {
      return;
    }

    match self.mode {
      Mode::HBlank => {},
      Mode::VBlank => {},
      Mode::OamScan => {

      },
      Mode::Drawing => {

      },
    }
  }
}
