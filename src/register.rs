#[derive(Clone, Copy, Debug, Default)]
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
  pub fn set_af(&mut self, val: u16) {
    self.a = (val >> 8) as u8;
    self.f = (val & 0xF0) as u8; // The lower 4 bits of F register are always 0s
  }
  #[inline]
  pub fn set_bc(&mut self, val: u16) {
    self.b = (val >> 8) as u8;
    self.c = val as u8;
  }
  #[inline]
  pub fn set_de(&mut self, val: u16) {
    self.d = (val >> 8) as u8;
    self.e = val as u8;
  }
  #[inline]
  pub fn set_hl(&mut self, val: u16) {
    self.h = (val >> 8) as u8;
    self.l = val as u8;
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
  pub fn set_zf(&mut self, zf: bool) {
    if zf {
      self.f |= 0b_1000_0000;
    } else {
      self.f &= 0b_0111_1111;
    }
  }
  #[inline]
  pub fn set_nf(&mut self, nf: bool) {
    if nf {
      self.f |= 0b_0100_0000;
    } else {
      self.f &= 0b_1011_1111;
    }
  }
  #[inline]
  pub fn set_hf(&mut self, hf: bool) {
    if hf {
      self.f |= 0b_0010_0000;
    } else {
      self.f &= 0b_1101_1111;
    }
  }
  #[inline]
  pub fn set_cf(&mut self, cf: bool) {
    if cf {
      self.f |= 0b_0001_0000;
    } else {
      self.f &= 0b_1110_1111;
    }
  }
}
