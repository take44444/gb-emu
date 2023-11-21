
use gbemu::{
  bootrom,
  cartridge,
  peripherals,
  cpu,
  joypad,
};
use std::{
  env,
  fs::File,
  io::Read,
  process::exit,
};

mod gameboy;
mod lcd;
mod audio;

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
  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    eprintln!("The file name argument is required.");
    exit(1);
  }
  // let bootrom_raw = file2vec(&args[1]);
  let cartridge_raw = file2vec(&args[1]);
  let save = if args.len() >= 4 { Some(file2vec(&args[2])) } else { None };

  let cartridge = cartridge::Cartridge::new(cartridge_raw.into(), save);
  let bootrom = bootrom::Bootrom::new(cartridge.is_cgb);

  let mut gameboy = gameboy::GameBoy::new(bootrom, cartridge);
  gameboy.run();
}
