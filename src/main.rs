use std::{
  env,
  fs::File,
  io::Read
};

mod cartridge;

fn main() {
  env_logger::init();

  let fname = env::args().nth(1).unwrap();
  let mut cartridge_raw = Vec::new();
  let mut file = File::open(fname).unwrap();
  file.read_to_end(&mut cartridge_raw).unwrap();
  let cartridge_header = cartridge::CartridgeHeader::new(&cartridge_raw).unwrap();
  println!("{:?}", cartridge_header);
}
