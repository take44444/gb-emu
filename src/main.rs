use std::{
  env,
  fs::File,
  io::Read,
  process::exit,
};

mod gameboy;
mod bootrom;
mod cartridge;
mod mbc;
mod joypad;
mod audio;
mod lcd;
mod interrupts;
mod peripherals;
mod cpu;
mod register;
mod ppu;
mod apu;
mod timer;
mod wram;
mod hram;
mod oam_dma;

fn file2vec(fname: &String) -> Vec<u8> {
  if let Ok(mut file) = File::open(fname) {
    let mut ret = vec![];
    file.read_to_end(&mut ret).unwrap();
    ret
  } else {
    panic!("Cannot open {}.", fname);
  }
}

fn main() {
  env_logger::init();

  let args: Vec<String> = env::args().collect();
  if args.len() < 3 {
    eprintln!("The file name argument is required.");
    exit(1);
  }
  let bootrom_raw = file2vec(&args[1]);
  let cartridge_raw = file2vec(&args[2]);
  let save = if args.len() >= 4 { Some(file2vec(&args[3])) } else { None };

  let bootrom = bootrom::Bootrom::new(bootrom_raw.into()).unwrap();
  let cartridge = cartridge::Cartridge::new(cartridge_raw.into(), save).unwrap();

  let mut gameboy = gameboy::GameBoy::new(bootrom, cartridge);
  gameboy.run().unwrap();
}
