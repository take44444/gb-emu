use std::{cell::RefCell, rc::Rc};

use crate::{interrupts, peripherals};

pub struct Bus {
  pub read: Box<dyn Fn(&interrupts::Interrupts, u16) -> u8>,
  pub write: Box<dyn Fn(&mut interrupts::Interrupts, u16, u8)>,
}

impl Bus {
  pub fn new(peripherals: Rc<RefCell<peripherals::Peripherals>>) -> Self {
    let p1 = peripherals.clone();
    let p2 = peripherals.clone();
    Self {
      read: Box::new(move |interrupts, addr| p1.borrow().read(interrupts, addr)),
      write: Box::new(move |interrupts, addr, val| p2.borrow_mut().write(interrupts, addr, val)),
    }
  }
}