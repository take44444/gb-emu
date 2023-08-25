use std::iter;

use sdl2::{
  pixels::PixelFormatEnum,
  render::Canvas,
  video::Window,
  Sdl,
};
use crate::ppu;

pub struct LCD(Canvas<Window>);

impl LCD {
  pub fn new(sdl_context: &Sdl, size: u32) -> LCD {
    let window = sdl_context.video().unwrap()
      .window("gb-emu", ppu::LCD_WIDTH as u32 * size, ppu::LCD_HEIGHT as u32 * size)
      .position_centered()
      .build()
      .unwrap();
    let canvas = window.into_canvas().build().unwrap();
    Self(canvas)
  }
  pub fn draw(&mut self, pixels: &Box<[ppu::Color; ppu::LCD_PIXELS]>) {
    let texture_creator = self.0.texture_creator();
    let mut texture = texture_creator
      .create_texture_streaming(PixelFormatEnum::RGB24, ppu::LCD_WIDTH as u32, ppu::LCD_HEIGHT as u32)
      .unwrap();

    texture.update(None, &pixels.iter().flat_map(
      |&e| iter::repeat(e.into()).take(3)
    ).collect::<Vec<u8>>(), 480).unwrap();
    self.0.clear();
    self.0.copy(&texture, None, None).unwrap();
    self.0.present();
  }
}
