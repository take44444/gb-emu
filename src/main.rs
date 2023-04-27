use std::{
  env,
  fs::File,
  io::Read,
  process::exit,
};

mod cartridge;
mod cpu;
mod gameboy;
mod ppu;

fn main() {
  env_logger::init();

  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    eprintln!("The file name argument is required.");
    exit(1);
  }
  let fname = &args[1];
  let mut file = if let Ok(f) = File::open(fname) {
    f
  } else {
    eprintln!("Cannot find {}.", fname);
    exit(1);
  };
  let mut cartridge_raw = Vec::new();
  file.read_to_end(&mut cartridge_raw).unwrap();
  let cartridge_header = cartridge::CartridgeHeader::new(&cartridge_raw).unwrap();
  println!("{:?}", cartridge_header);
}
