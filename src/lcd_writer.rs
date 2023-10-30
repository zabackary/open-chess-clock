use core::cell::RefCell;

use arduino_hal::Delay;
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uWrite;

pub struct LcdWriter<'a, B: DataBus>(&'a RefCell<HD44780<B>>);

impl<'a, B: DataBus> LcdWriter<'a, B> {
    pub fn new(lcd: &'a RefCell<HD44780<B>>) -> LcdWriter<'a, B> {
        LcdWriter(lcd)
    }
}

impl<B: DataBus> uWrite for LcdWriter<'_, B> {
    type Error = hd44780_driver::error::Error;
    fn write_str(&mut self, s: &str) -> hd44780_driver::error::Result<()> {
        let mut d = Delay::new();
        self.0
            .try_borrow_mut()
            .map_err(|_| hd44780_driver::error::Error)?
            .write_str(s, &mut d)
    }
}
