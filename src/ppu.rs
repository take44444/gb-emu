pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const SCREEN_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum Color {
  Off = 0,
  Light = 1,
  Dark = 2,
  On = 3,
}

pub type PixelBuffer = [Color; SCREEN_PIXELS];

pub struct Ppu {
  pub pixel_buffer: PixelBuffer,
}

impl Ppu {
  pub fn new() -> Self {
    Self {
      pixel_buffer: [Color::Off; SCREEN_PIXELS],
    }
  }

  pub fn emulate_cycle(&mut self) {
    
  }

  pub fn get_vblank_event(&self) -> bool {
    true
  }
}
