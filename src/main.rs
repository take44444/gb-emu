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
mod timer;
mod register;
mod interrupts;
mod oam_dma;
mod lcd;

fn file2vec(fname: &String) -> Vec<u8> {
  let mut file = if let Ok(f) = File::open(fname) {
    f
  } else {
    eprintln!("Cannot find {}.", fname);
    exit(1);
  };
  let mut ret = vec![];
  file.read_to_end(&mut ret).unwrap();
  ret
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

  let bootrom = bootrom::Bootrom::new(bootrom_raw.into()).unwrap();
  let cartridge = cartridge::Cartridge::new(cartridge_raw.into()).unwrap();

  let mut gameboy = gameboy::GameBoy::new(bootrom, cartridge);
  gameboy.run().unwrap();
}
