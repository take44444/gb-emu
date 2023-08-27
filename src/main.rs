use std::{
  env,
  fs::File,
  io::Read,
  process::exit,
};

mod peripherals;
mod bootrom;
mod cartridge;
mod mbc;
mod cpu;
mod gameboy;
mod wram;
mod hram;
mod ppu;
// mod apu;
mod timer;
mod register;
mod interrupts;
mod oam_dma;
mod lcd;
mod joypad;

fn file2vec(fname: &String) -> Vec<u8> {
  if let Ok(mut file) = File::open(fname) {
    let mut ret = vec![];
    file.read_to_end(&mut ret).unwrap();
    ret
  } else {
    panic!("Cannot find {}.", fname);
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
