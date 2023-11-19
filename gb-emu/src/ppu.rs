use std::cmp::min;

use crate::{
  LCD_WIDTH,
  LCD_PIXELS,
  cpu::interrupts::{self, Interrupts},
};

const BG_WINDOW_ENABLE: u8 = 1 << 0;
const SPRITE_ENABLE: u8 = 1 << 1;
const SPRITE_SIZE: u8 = 1 << 2;
const BG_TILE_MAP: u8 = 1 << 3;
const TILE_DATA_ADDRESSING_MODE: u8 = 1 << 4;
const WINDOW_ENABLE: u8 = 1 << 5;
const WINDOW_TILE_MAP: u8 = 1 << 6;
const PPU_ENABLE: u8 = 1 << 7;

const LYC_EQ_LY: u8 = 1 << 2;
const HBLANK_INT: u8 = 1 << 3;
const VBLANK_INT: u8 = 1 << 4;
const OAM_SCAN_INT: u8 = 1 << 5;
const LYC_EQ_LY_INT: u8 = 1 << 6;

const BANK: u8 = 1 << 3;
const PALETTE: u8 = 1 << 4;
const X_FLIP: u8 = 1 << 5;
const Y_FLIP: u8 = 1 << 6;
const OBJ2BG_PRIORITY: u8 = 1 << 7;

#[derive(Copy, Clone, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct Ppu {
  is_cgb: bool,
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
  wly: u8,
  vram: Box<[u8; 0x2000]>,
  bcps: u8,
  ocps: u8,
  vbk: u8,
  vram2: Box<[u8; 0x2000]>,
  oam: Box<[u8; 0xA0]>,
  pub oam_dma: Option<u16>,
  pub hdma_src: u16,
  hdma_dst: u16,
  pub hblank_dma: Option<u16>,
  pub general_dma: Option<u16>,
  bg_palette_memory: Box<[u8; 0x40]>,
  sprite_palette_memory: Box<[u8; 0x40]>,
  cycles: u8,
  buffer: Box<[u8; LCD_PIXELS*4]>,
}

impl Ppu {
  pub fn new(is_cgb: bool) -> Self {
    Self {
      is_cgb,
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
      wly: 0,
      vram: Box::new([0; 0x2000]),
      bcps: 0,
      ocps: 0,
      vbk: 0,
      vram2: Box::new([0; 0x2000]),
      oam: Box::new([0; 0xA0]),
      oam_dma: None,
      hdma_src: 0,
      hdma_dst: 0,
      hblank_dma: None,
      general_dma: None,
      bg_palette_memory: Box::new([0; 0x40]),
      sprite_palette_memory: Box::new([0; 0x40]),
      cycles: 20,
      buffer: Box::new([0; LCD_PIXELS*4]),
    }
  }
  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0x8000..=0x9FFF => if self.mode == Mode::Drawing {
        0xFF
      } else {
        if self.vbk & 1 > 0 {
          self.vram2[addr as usize & 0x1FFF]
        } else {
          self.vram[addr as usize & 0x1FFF]
        }
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
      0xFF4F          => self.vbk | 0xFE,
      0xFF55          => if let Some(len) = self.hblank_dma {
        0x80 | (((len >> 4) - 1) & 0x7F) as u8
      } else {
        0xFF
      },
      0xFF68          => self.bcps,
      0xFF69          => if self.mode == Mode::Drawing {
        0xFF
      } else {
        self.bg_palette_memory[self.bcps as usize & 0x3F]
      },
      0xFF6A          => self.ocps,
      0xFF6B          => if self.mode == Mode::Drawing {
        0xFF
      } else {
        self.sprite_palette_memory[self.ocps as usize & 0x3F]
      },
      _               => unreachable!(),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match addr {
      0x8000..=0x9FFF => if self.mode != Mode::Drawing {
        if self.vbk & 1 > 0 {
          self.vram2[addr as usize & 0x1FFF] = val;
        } else {
          self.vram[addr as usize & 0x1FFF] = val;
        }
      },
      0xFE00..=0xFE9F => if self.mode != Mode::Drawing && self.mode != Mode::OamScan {
        if self.oam_dma.is_none() {
          self.oam[addr as usize & 0xFF] = val;
        }
      },
      0xFF40          => self.lcdc = val,
      0xFF41          => self.stat = (self.stat & LYC_EQ_LY) | (val & 0xF8),
      0xFF42          => self.scy = val,
      0xFF43          => self.scx = val,
      0xFF44          => {},
      0xFF45          => self.lyc = val,
      0xFF46          => {
        assert!(val <= 0xDF);
        self.oam_dma = Some((val as u16) << 8);
      },
      0xFF47          => self.bgp = val,
      0xFF48          => self.obp0 = val,
      0xFF49          => self.obp1 = val,
      0xFF4A          => self.wy = val,
      0xFF4B          => self.wx = val,
      0xFF4F          => self.vbk = val,
      0xFF51          => self.hdma_src = (self.hdma_src & 0xF0) | (val as u16 & 0xFF) << 8,
      0xFF52          => self.hdma_src = (self.hdma_src & 0xFF00) | (val as u16 & 0xF0),
      0xFF53          => self.hdma_dst = (self.hdma_dst & 0xF0) | (val as u16 & 0x1F) << 8,
      0xFF54          => self.hdma_dst = (self.hdma_dst & 0x1F00) | (val as u16 & 0xF0),
      0xFF55          => if val & 0x80 > 0 {
        self.hblank_dma = Some(min(
          0x2000 - self.hdma_dst,
          ((val as u16 & 0x7F) + 1) << 4)
        );
      } else {
        self.general_dma = Some(min(
          0x2000 - self.hdma_dst,
          ((val as u16 & 0x7F) + 1) << 4)
        );
        self.hblank_dma = None;
      }
      0xFF68          => self.bcps = val,
      0xFF69          => {
        if self.mode != Mode::Drawing {
          self.bg_palette_memory[self.bcps as usize & 0x3F] = val;
        }
        if self.bcps & 0x80 > 0 {
          self.bcps = (self.bcps & 0xC0) | (((self.bcps & 0x3F) + 1) & 0x3F);
        }
      },
      0xFF6A          => self.ocps = val,
      0xFF6B          => {
        if self.mode != Mode::Drawing {
          self.sprite_palette_memory[self.ocps as usize & 0x3F] = val;
        }
        if self.ocps & 0x80 > 0 {
          self.ocps = (self.ocps & 0xC0) | (((self.ocps & 0x3F) + 1) & 0x3F);
        }
      },
      _               => unreachable!(),
    }
  }
  pub fn emulate_cycle(&mut self, interrupts: &mut Interrupts) -> bool {
    if self.lcdc & PPU_ENABLE == 0 {
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
          if self.stat & OAM_SCAN_INT > 0 {
            interrupts.irq(interrupts::STAT);
          }
        } else {
          self.mode = Mode::VBlank;
          self.cycles = 114;
          interrupts.irq(interrupts::VBLANK);
          if self.stat & VBLANK_INT > 0 {
            interrupts.irq(interrupts::STAT);
          }
        }
        self.check_lyc_eq_ly(interrupts);
      },
      Mode::VBlank => {
        self.ly += 1;
        if self.ly > 153 {
          ret = true;
          self.ly = 0;
          self.wly = 0;
          self.mode = Mode::OamScan;
          self.cycles = 20;
          if self.stat & OAM_SCAN_INT > 0 {
            interrupts.irq(interrupts::STAT);
          }
        } else {
          self.cycles = 114;
        }
        self.check_lyc_eq_ly(interrupts);
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
          interrupts.irq(interrupts::STAT);
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
  pub fn hblank_dma_emulate_cycle(&mut self, vals: [u8; 10]) {
    if let Some(len) = self.hblank_dma {
      if self.mode == Mode::HBlank && self.cycles == 51 {
        assert!(len >= 0x10);
        assert!(self.hdma_dst + 0x10 <= 0x2000);
        for i in 0..0x10 {
          if self.vbk & 1 > 0 {
            self.vram2[self.hdma_dst as usize + i] = vals[i];
          } else {
            self.vram[self.hdma_dst as usize + i] = vals[i];
          }
        }
        self.hdma_src += 0x10;
        self.hdma_dst += 0x10;
        self.hblank_dma = Some(len.wrapping_sub(0x10)).filter(|&x| x > 0);
        if self.hdma_dst >= 0x2000 {
          self.hblank_dma = None;
        }
      }
    }
  }
  pub fn general_dma_emulate_cycle(&mut self, vals: Vec<u8>) {
    if let Some(len) = self.general_dma {
      assert!(len as usize == vals.len());
      assert!(self.hdma_dst + len <= 0x2000);
      for i in 0..len as usize {
        if self.vbk & 1 > 0 {
          self.vram2[self.hdma_dst as usize + i] = vals[i];
        } else {
          self.vram[self.hdma_dst as usize + i] = vals[i];
        }
      }
      self.general_dma = None;
    }
  }
  pub fn pixel_buffer(&self) -> Box<[u8]> {
    self.buffer.clone()
  }
  fn render(&mut self) {
    if self.is_cgb {
      let mut bg_prio: [(bool, bool); LCD_WIDTH] = [(false, false); LCD_WIDTH];
      self.render_bg_cgb(&mut bg_prio);
      self.render_window_cgb(&mut bg_prio);
      self.render_sprite_cgb(&bg_prio);
    } else {
      let mut bg_prio: [bool; LCD_WIDTH] = [false; LCD_WIDTH];
      self.render_bg(&mut bg_prio);
      self.render_window(&mut bg_prio);
      self.render_sprite(&bg_prio);
    }
  }
  fn render_bg(&mut self, bg_prio: &mut [bool; LCD_WIDTH]) {
    if self.lcdc & BG_WINDOW_ENABLE == 0 {
      return;
    }
    let y = self.ly.wrapping_add(self.scy);
    for i in 0..LCD_WIDTH {
      let x = (i as u8).wrapping_add(self.scx);
      let tile_idx = self.get_tile_idx_from_tile_map(
        self.lcdc & BG_TILE_MAP > 0,
        y >> 3, x >> 3
      );
      let pixel = self.get_pixel_from_tile(tile_idx, y & 7, x & 7, false);
      self.buffer[LCD_WIDTH * self.ly as usize + i] = 
        match (self.bgp >> (pixel << 1)) & 0b11 {
          0b00 => 0xFF,
          0b01 => 0xAA,
          0b10 => 0x55,
          _    => 0x00,
        };
      bg_prio[i] = pixel > 0;
    }
  }
  fn render_window(&mut self, bg_prio: &mut [bool; LCD_WIDTH]) {
    if self.lcdc & BG_WINDOW_ENABLE == 0 || self.lcdc & WINDOW_ENABLE == 0 || self.wy > self.ly {
      return;
    }
    let mut wly_add = 0;
    let y = self.wly;
    for i in 0..LCD_WIDTH {
      let (x, overflow) = (i as u8).overflowing_sub(self.wx.wrapping_sub(7));
      if overflow {
        continue;
      }
      wly_add = 1;
      let tile_idx = self.get_tile_idx_from_tile_map(
        (self.lcdc & WINDOW_TILE_MAP) > 0,
        y >> 3, x >> 3
      );
      let pixel = self.get_pixel_from_tile(tile_idx, y & 7, x & 7, false);
      self.buffer[LCD_WIDTH * self.ly as usize + i] = 
        match (self.bgp >> (pixel << 1)) & 0b11 {
          0b00 => 0xFF,
          0b01 => 0xAA,
          0b10 => 0x55,
          _    => 0x00,
        };
      bg_prio[i] = pixel > 0;
    }
    self.wly += wly_add;
  }
  fn render_sprite(&mut self, bg_prio: &[bool; LCD_WIDTH]) {
    if self.lcdc & SPRITE_ENABLE == 0 {
      return;
    }
    let size = if self.lcdc & SPRITE_SIZE > 0 { 16 } else { 8 };

    let mut sprites: Vec<Sprite> = unsafe {
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
    }).take(10).collect();
    sprites.reverse();
    sprites.sort_by(|&a, &b| b.x.cmp(&a.x));

    for sprite in sprites {
      let palette = if sprite.flags & PALETTE > 0 { self.obp1 } else { self.obp0 };
      let mut tile_idx = sprite.tile_idx as usize;
      let mut row = if sprite.flags & Y_FLIP > 0 {
        size - 1 - self.ly.wrapping_sub(sprite.y)
      } else {
        self.ly.wrapping_sub(sprite.y)
      };

      // if the size is 16 and it is second tile
      if size == 16 {
        tile_idx &= 0xFE;
      }
      tile_idx += (row >= 8) as usize;
      row &= 7;

      for col in 0..8 {
        let col_flipped = if sprite.flags & X_FLIP > 0 {
          7 - col
        } else {
          col
        };
        let pixel = self.get_pixel_from_tile(tile_idx, row, col_flipped, false);
        let i = sprite.x.wrapping_add(col) as usize;
        if i < LCD_WIDTH && pixel > 0 {
          if sprite.flags & OBJ2BG_PRIORITY == 0 || !bg_prio[i] {
            self.buffer[LCD_WIDTH * self.ly as usize + i] = 
              match (palette >> (pixel << 1)) & 0b11 {
                0b00 => 0xFF,
                0b01 => 0xAA,
                0b10 => 0x55,
                _    => 0x00,
              };
          }
        }
      }
    }
  }
  fn render_bg_cgb(&mut self, bg_prio: &mut [(bool, bool); LCD_WIDTH]) {
    let y = self.ly.wrapping_add(self.scy);
    for i in 0..LCD_WIDTH {
      let x = (i as u8).wrapping_add(self.scx);
      let tile_idx = self.get_tile_idx_from_tile_map(
        self.lcdc & BG_TILE_MAP > 0,
        y >> 3, x >> 3
      );
      let attr = self.get_bg_attr(
        self.lcdc & BG_TILE_MAP > 0,
        y >> 3, x >> 3
      );
      let palette = attr & 0b111;
      let row = if attr & Y_FLIP > 0 {
        7 - (y & 7)
      } else {
        y & 7
      };
      let col = if attr & X_FLIP > 0 {
        7 - (x & 7)
      } else {
        x & 7
      };
      let pixel = self.get_pixel_from_tile(tile_idx, row, col, attr & BANK > 0);
      let color = self.get_color_from_palette_memory(palette, pixel, false);
      for j in 0..4 {
        self.buffer[(LCD_WIDTH * self.ly as usize + i) * 4 + j] = (color[j] << 3) | (color[j] >> 2);
        bg_prio[i] = (attr & OBJ2BG_PRIORITY > 0, pixel > 0);
      }
    }
  }
  fn render_window_cgb(&mut self, bg_prio: &mut [(bool, bool); LCD_WIDTH]) {
    if self.lcdc & WINDOW_ENABLE == 0 || self.wy > self.ly {
      return;
    }
    let mut wly_add = 0;
    let y = self.wly;
    for i in 0..LCD_WIDTH {
      let (x, overflow) = (i as u8).overflowing_sub(self.wx.wrapping_sub(7));
      if overflow {
        continue;
      }
      wly_add = 1;
      let tile_idx = self.get_tile_idx_from_tile_map(
        (self.lcdc & WINDOW_TILE_MAP) > 0,
        y >> 3, x >> 3
      );
      let attr = self.get_bg_attr(
        self.lcdc & WINDOW_TILE_MAP > 0,
        y >> 3, x >> 3
      );
      let palette = attr & 0b111;
      let row = if attr & Y_FLIP > 0 {
        7 - (y & 7)
      } else {
        y & 7
      };
      let col = if attr & X_FLIP > 0 {
        7 - (x & 7)
      } else {
        x & 7
      };
      let pixel = self.get_pixel_from_tile(tile_idx, row, col, attr & BANK > 0);
      let color = self.get_color_from_palette_memory(palette, pixel, false);
      for j in 0..4 {
        self.buffer[(LCD_WIDTH * self.ly as usize + i) * 4 + j] = (color[j] << 3) | (color[j] >> 2);
        bg_prio[i] = (attr & OBJ2BG_PRIORITY > 0, pixel > 0);
      }
    }
    self.wly += wly_add;
  }
  fn render_sprite_cgb(&mut self, bg_prio: &[(bool, bool); LCD_WIDTH]) {
    if self.lcdc & SPRITE_ENABLE == 0 {
      return;
    }
    let size = if self.lcdc & SPRITE_SIZE > 0 { 16 } else { 8 };

    let mut sprites: Vec<Sprite> = unsafe {
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
    }).take(10).collect();
    sprites.reverse();

    for sprite in sprites {
      let palette = sprite.flags & 0b111;
      let mut tile_idx = sprite.tile_idx as usize;
      let mut row = if sprite.flags & Y_FLIP > 0 {
        size - 1 - self.ly.wrapping_sub(sprite.y)
      } else {
        self.ly.wrapping_sub(sprite.y)
      };

      // if the size is 16 and it is second tile
      if size == 16 {
        tile_idx &= 0xFE;
      }
      tile_idx += (row >= 8) as usize;
      row &= 7;

      for col in 0..8 {
        let col_flipped = if sprite.flags & X_FLIP > 0 {
          7 - col
        } else {
          col
        };
        let pixel = self.get_pixel_from_tile(tile_idx, row, col_flipped, sprite.flags & BANK > 0);
        let i = sprite.x.wrapping_add(col) as usize;
        if i < LCD_WIDTH && pixel > 0 {
          if self.lcdc & BG_WINDOW_ENABLE == 0 ||
            ((sprite.flags & OBJ2BG_PRIORITY == 0) && !bg_prio[i].0) ||
            !bg_prio[i].1
          {
            let color = self.get_color_from_palette_memory(palette, pixel, true);
            for j in 0..4 {
              self.buffer[(LCD_WIDTH * self.ly as usize + i) * 4 + j] = (color[j] << 3) | (color[j] >> 2);
            }
          }
        }
      }
    }
  }
  fn get_tile_idx_from_tile_map(&self, tile_map: bool, row: u8, col: u8) -> usize {
    let start_addr: usize = 0x1800 | ((tile_map as usize) << 10);
    let ret = self.vram[start_addr | ((((row as usize) << 5) + col as usize) & 0x3FF)];
    if self.lcdc & TILE_DATA_ADDRESSING_MODE > 0 {
      ret as usize
    } else {
      ((ret as i8 as i16) + 0x100) as usize
    }
  }
  fn get_pixel_from_tile(&self, tile_idx: usize, row: u8, col: u8, from2: bool) -> u8 {
    let vram = if from2 {
      &self.vram2
    } else {
      &self.vram
    };
    let r = (row << 1) as usize;
    let c = (7 - col) as usize;
    let tile_addr = tile_idx << 4;
    let low = vram[(tile_addr | r) & 0x1FFF];
    let high = vram[(tile_addr | (r + 1)) & 0x1FFF];
    (((high >> c) & 1) << 1) | ((low >> c) & 1)
  }
  fn get_bg_attr(&self, tile_map: bool, row: u8, col: u8) -> u8 {
    let start_addr: usize = 0x1800 | ((tile_map as usize) << 10);
    self.vram2[start_addr | ((((row as usize) << 5) + col as usize) & 0x3FF)]
  }
  fn get_color_from_palette_memory(&self, palette: u8, pixel: u8, is_sprite: bool) -> [u8; 4] {
    let mut rgba = [0xFF; 4];
    let palette_memory = if is_sprite {
      &self.sprite_palette_memory
    } else {
      &self.bg_palette_memory
    };
    let rgb555 = 
      (palette_memory[((palette as usize) << 3) + ((pixel as usize) << 1)] as u16) |
      (palette_memory[((palette as usize) << 3) + ((pixel as usize) << 1) + 1] as u16);
    rgba[0] = (rgb555         & 0x1F) as u8;
    rgba[1] = ((rgb555 >> 5)  & 0x1F) as u8;
    rgba[2] = ((rgb555 >> 10) & 0x1F) as u8;
    rgba
  }
  fn check_lyc_eq_ly(&mut self, interrupts: &mut Interrupts) {
    if self.ly == self.lyc {
      self.stat |= LYC_EQ_LY;
      if self.stat & LYC_EQ_LY_INT > 0 {
        interrupts.irq(interrupts::STAT);
      }
    } else {
      self.stat &= !LYC_EQ_LY;
    }
  }
}
