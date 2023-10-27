#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mbc {
  NoMbc,
  Mbc1 {
    sram_enable: bool,
    low_bank: usize,
    high_bank: usize,
    bank_mode: bool,
    rom_banks: usize, // ROMのバンク数
  },
}

impl Mbc {
  pub fn new(cartridge_type: u8, rom_banks: usize) -> Self {
    match cartridge_type {
      0x00 | 0x08 | 0x09 => Self::NoMbc,
      0x01..=0x03        => Self::Mbc1 {
        sram_enable: false,
        low_bank: 0b00001, // 1で初期化する必要がある
        high_bank: 0b00,
        bank_mode: false,
        rom_banks,
      },
      _                  => panic!("Not supported: {:02x}", cartridge_type),
    }
  }
  pub fn write(&mut self, addr: u16, val: u8) {
    match self {
      Self::NoMbc => {},
      Self::Mbc1 {
        ref mut sram_enable,
        ref mut low_bank,
        ref mut high_bank,
        ref mut bank_mode,
        ..
      } => match addr {
        0x0000..=0x1FFF => *sram_enable = val & 0xF == 0xA,
        0x2000..=0x3FFF => *low_bank = if val & 0b11111 == 0b00000 {
          0b00001 // 下位5bitが全て0の場合は代わりに0b00001が書き込まれる
        } else {
          (val & 0b11111) as usize
        },
        0x4000..=0x5FFF => *high_bank = (val & 0b11) as usize,
        0x6000..=0x7FFF => *bank_mode = val & 0b1 > 0,
        _ => unreachable!(),
      },
    }
  }
  pub fn get_addr(&self, addr: u16) -> usize {
    match self {
      Self::NoMbc => addr as usize,
      Self::Mbc1 {
        low_bank,
        high_bank,
        bank_mode,
        rom_banks,
        ..
      } => match addr {
        0x0000..=0x3FFF => if *bank_mode {
          (*high_bank << 19) | (addr & 0x3FFF) as usize
        } else {
          (addr & 0x3FFF) as usize
        },
        0x4000..=0x7FFF => (*high_bank << 19) | ((low_bank & (rom_banks - 1)) << 14) | (addr & 0x3FFF) as usize,
        0xA000..=0xBFFF => if *bank_mode {
          (*high_bank << 13) | (addr & 0x1FFF) as usize
        } else {
          (addr & 0x1FFF) as usize
        },
        _               => unreachable!(),
      },
    }
  }
}