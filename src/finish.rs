use core::cell::RefCell;

use arduino_hal::{delay_ms, Delay};
use embedded_hal::digital::v2::InputPin;
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uwrite;

use crate::{countdown::Turn, lcd_writer::LcdWriter};

const LOOP_DELAY: u16 = 5;

pub fn finish<SP: InputPin, B: DataBus>(
    loser: &Turn,
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
    start_pin: &mut SP,
) -> Result<(), hd44780_driver::error::Error> {
    lcd.borrow_mut().set_cursor_pos(0, delay)?;
    if *loser == Turn::P1 {
        uwrite!(writer, "[P1]  Time's up!")?;
    } else {
        uwrite!(writer, "Time's up!  [P2]")?;
    }
    let mut start = debouncr::debounce_4(false);
    loop {
        if start.update(
            start_pin
                .is_low()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Rising)
        {
            // Start press; continue
            break;
        }
        delay_ms(LOOP_DELAY);
    }
    Ok(())
}
