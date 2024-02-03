use core::cell::RefCell;

use arduino_hal::{delay_ms, hal::Atmega, usart::UsartOps, Delay};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uwrite;

use crate::{countdown::Turn, error::RuntimeError, lcd_writer::LcdWriter, serial::SerialHandler};

const LOOP_DELAY: u16 = 5;
const BUZZER_LENGTH: u16 = 120;

pub fn finish<SP: InputPin, BP: OutputPin, B: DataBus, USART: UsartOps<Atmega, RX, TX>, RX, TX>(
    loser: &Turn,
    _serial_handler: &mut SerialHandler<USART, RX, TX>,
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
    start_pin: &mut SP,
    buzzer_pin: &mut BP,
) -> Result<(), RuntimeError> {
    buzzer_pin
        .set_high()
        .map_err(|_| RuntimeError::PinWriteError)?;

    lcd.borrow_mut()
        .set_cursor_pos(0, delay)
        .map_err(|_| RuntimeError::LcdError)?;
    if *loser == Turn::P1 {
        uwrite!(writer, "[P1]  Time's up!").map_err(|_| RuntimeError::LcdError)?;
    } else {
        uwrite!(writer, "Time's up!  [P2]").map_err(|_| RuntimeError::LcdError)?;
    }
    let mut start = debouncr::debounce_4(false);
    let mut i = 0;
    loop {
        if i < BUZZER_LENGTH {
            i += 1;
        } else {
            buzzer_pin
                .set_low()
                .map_err(|_| RuntimeError::PinWriteError)?;
        }
        if start.update(start_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
        {
            // Start press; continue
            break;
        }
        delay_ms(LOOP_DELAY);
    }
    Ok(())
}
