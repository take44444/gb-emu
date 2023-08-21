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
  White = 0,
  LightGray = 1,
  DarkGray = 2,
  Black = 3,
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
  vram: Box<[u8; 0x2000]>,
  oam: Box<[u8; 0x100]>,
  cycles: u8,
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
      vram: Box::new([0; 0x2000]),
      oam: Box::new([0; 0x100]),
      cycles: 0,
    }
  }

  fn change_mode(&mut self, interrupts: &mut interrupts::Interrupts) {
    match self.mode {
      Mode::HBlank => {
        self.mode = Mode::VBlank;
        self.cycles = 114;
      },
      Mode::VBlank => {
        self.mode = Mode::OamScan;
        self.cycles = 21;
      },
      Mode::OamScan => {
        self.mode = Mode::Drawing;
        // self.cycles = ?;
      },
      Mode::Drawing => {
        self.mode = Mode::HBlank;
        // self.cycles = ?;
      },
    }
  }

  pub fn emulate_cycle(&mut self, interrupts: &mut interrupts::Interrupts) -> bool {
    if !self.lcdc & LCD_DISPLAY_ENABLE > 0 {
      return false;
    }

    self.cycles -= 1;
    if self.cycles > 0 {
      return false;
    }

    match self.mode {
      Mode::HBlank => self.change_mode(interrupts),
      Mode::VBlank => self.change_mode(interrupts),
      Mode::OamScan => self.change_mode(interrupts),
      Mode::Drawing => {
        self.draw();
        self.change_mode(interrupts);
      },
    }
    return false;
  }

  fn draw(&mut self) {

  }
}
