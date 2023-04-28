#[derive(Clone, Copy, Debug)]
pub struct Registers {
  pub pc: u16,
  pub sp: u16,
  pub a: u8,
  pub f: u8,
  pub b: u8,
  pub c: u8,
  pub d: u8,
  pub e: u8,
  pub h: u8,
  pub l: u8,
}

impl Registers {
  pub fn new() -> Registers {
    Registers {
      pc: 0,
      sp: 0,
      a: 0,
      f: 0,
      b: 0,
      c: 0,
      d: 0,
      e: 0,
      h: 0,
      l: 0,
    }
  }
  #[inline]
  pub fn af(&self) -> u16 {
    ((self.a as u16) << 8) | (self.f as u16)
  }
  #[inline]
  pub fn bc(&self) -> u16 {
    ((self.b as u16) << 8) | (self.c as u16)
  }
  #[inline]
  pub fn de(&self) -> u16 {
    ((self.d as u16) << 8) | (self.e as u16)
  }
  #[inline]
  pub fn hl(&self) -> u16 {
    ((self.h as u16) << 8) | (self.l as u16)
  }

  #[inline]
  pub fn zf(&self) -> bool {
    (self.f & 0b_1000_0000) > 0
  }
  #[inline]
  pub fn nf(&self) -> bool {
    (self.f & 0b_0100_0000) > 0
  }
  #[inline]
  pub fn hf(&self) -> bool {
    (self.f & 0b_0010_0000) > 0
  }
  #[inline]
  pub fn cf(&self) -> bool {
    (self.f & 0b_0001_0000) > 0
  }
  #[inline]
  pub fn set_zf(&self, zf: bool) -> bool {
    if zf {
      self.f |= 0b_1000_0000;
    } else {
      self.f &= 0b_0111_1111;
    }
  }
  #[inline]
  pub fn set_nf(&self, nf: bool) -> bool {
    if nf {
      self.f |= 0b_0100_0000;
    } else {
      self.f &= 0b_1011_1111;
    }
  }
  #[inline]
  pub fn set_hf(&self, hf: bool) -> bool {
    if hf {
      self.f |= 0b_0010_0000;
    } else {
      self.f &= 0b_1101_1111;
    }
  }
  #[inline]
  pub fn set_cf(&self, cf: bool) -> bool {
    if cf {
      self.f |= 0b_0001_0000;
    } else {
      self.f &= 0b_1110_1111;
    }
  }
}
