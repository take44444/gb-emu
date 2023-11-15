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
  Mbc3 {
    sram_enable: bool,
    low_bank: usize,
    high_bank: usize,
    rtc_mode: bool,
    has_rtc: bool,
    rom_banks: usize,
  },
  Mbc5 {
    sram_enable: bool,
    low_bank: usize,
    high_bank: usize,
    rom_banks: usize,
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
      0x0f..=0x13       => Self::Mbc3 {
        sram_enable: false,
        low_bank: 1,
        high_bank: 0b00,
        rtc_mode: false,
        has_rtc: cartridge_type <= 0x10,
        rom_banks,
      },
      0x19..=0x1e       => Self::Mbc5 {
        sram_enable: false,
        low_bank: 1,
        high_bank: 0b00,
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
        0x0000..=0x1fff => *sram_enable = val & 0xf == 0xa,
        0x2000..=0x3fff => *low_bank = if val & 0b11111 == 0b00000 {
          0b00001 // 下位5bitが全て0の場合は代わりに0b00001が書き込まれる
        } else {
          (val & 0b11111) as usize
        },
        0x4000..=0x5fff => *high_bank = (val & 0b11) as usize,
        0x6000..=0x7fff => *bank_mode = val & 1 > 0,
        _ => (),
      },
      Self::Mbc3 {
        ref mut sram_enable,
        ref mut low_bank,
        ref mut high_bank,
        ref mut rtc_mode,
        ref mut has_rtc,
        ..
      } => match addr {
        0x0000..=0x1fff => *sram_enable = val & 0xf == 0xa,
        0x2000..=0x3fff => *low_bank = if val == 0 {
          1
        } else {
          (val & 0x7f) as usize
        },
        0x4000..=0x5fff => if val < 4 {
          *rtc_mode = false;
          *high_bank = (val & 0b11) as usize;
        } else if val >= 0x8 && val <= 0xc && *has_rtc {
          *rtc_mode = true;
        },
        0x6000..=0x7fff => (),
        _ => (),
      },
      Self::Mbc5 {
        ref mut sram_enable,
        ref mut low_bank,
        ref mut high_bank,
        ..
      } => match addr {
        0x0000..=0x1fff => *sram_enable = val & 0xf == 0xa,
        0x2000..=0x2fff => *low_bank = (*low_bank & 0x100) | val as usize,
        0x3000..=0x3fff => *low_bank = (*low_bank & 0x0ff) | ((val as usize & 1) << 8),
        0x4000..=0x5fff => *high_bank = (val & 0xf) as usize,
        _ => (),
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
        0x0000..=0x3fff => if *bank_mode {
          (*high_bank << 19) | (addr & 0x3fff) as usize
        } else {
          (addr & 0x3fff) as usize
        },
        0x4000..=0x7fff => (*high_bank << 19) | ((low_bank & (rom_banks - 1)) << 14) | (addr & 0x3fff) as usize,
        0xa000..=0xbfff => if *bank_mode {
          (*high_bank << 13) | (addr & 0x1fff) as usize
        } else {
          (addr & 0x1fff) as usize
        },
        _               => 0xff,
      },
      Self::Mbc3 {
        low_bank,
        high_bank,
        rom_banks,
        ..
      } => match addr {
        0x0000..=0x3fff => (addr & 0x3fff) as usize,
        0x4000..=0x7fff => ((low_bank & (rom_banks - 1)) << 14) | (addr & 0x3fff) as usize,
        0xa000..=0xbfff => (*high_bank << 13) | (addr & 0x1fff) as usize,
        _               => 0xff,
      },
      Self::Mbc5 {
        low_bank,
        high_bank,
        rom_banks,
        ..
      } => match addr {
        0x0000..=0x3fff => (addr & 0x3fff) as usize,
        0x4000..=0x7fff => ((low_bank & (rom_banks - 1)) << 14) | (addr & 0x3fff) as usize,
        0xa000..=0xbfff => (*high_bank << 13) | (addr & 0x1fff) as usize,
        _               => 0xff,
      },
    }
  }
}