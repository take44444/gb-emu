pub const CPU_CLOCK_HZ: u128 = 4_194_304;

pub const SAMPLES: usize = 512;
pub const SAMPLE_RATE: u128 = 48000;

pub const LCD_WIDTH: usize = 160;
pub const LCD_HEIGHT: usize = 144;
pub const LCD_PIXELS: usize = LCD_WIDTH * LCD_HEIGHT;

pub mod gameboy;
pub mod joypad;
mod apu;
mod bootrom;
mod cartridge;
mod cpu;
mod peripherals;
mod ppu;
mod serial;
mod timer;
mod hram;
mod wram;