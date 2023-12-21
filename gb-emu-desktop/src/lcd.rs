use sdl2::{
  pixels::PixelFormatEnum,
  render::Canvas,
  video::Window,
  Sdl,
};

use gbemu::{LCD_WIDTH, LCD_HEIGHT};

pub struct LCD(Canvas<Window>);

impl LCD {
  pub fn new(sdl: &Sdl, scale: u32) -> LCD {
    let window = sdl.video().expect("failed to initialize SDL video subsystem")
      .window("gb-emu", LCD_WIDTH as u32 * scale, LCD_HEIGHT as u32 * scale)
      .position_centered()
      .resizable()
      .build()
      .expect("failed to create a window");
    let canvas = window.into_canvas().build().unwrap();
    Self(canvas)
  }
  pub fn draw(&mut self, pixels: &Vec<u8>) {
    let texture_creator = self.0.texture_creator();
    let mut texture = texture_creator
      .create_texture_streaming(PixelFormatEnum::RGBA32, LCD_WIDTH as u32, LCD_HEIGHT as u32)
      .unwrap();

    texture.update(None, pixels, 640).unwrap();
    self.0.clear();
    self.0.copy(&texture, None, None).unwrap();
    self.0.present();
  }
  pub fn resize(&mut self, width: u32, _: u32) {
    self.0.set_logical_size(width, width * LCD_HEIGHT as u32 / LCD_WIDTH as u32).unwrap();
  }
}
