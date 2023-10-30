use core::cell::RefCell;

use arduino_hal::{delay_ms, Delay};
use embedded_hal::digital::v2::InputPin;
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uwrite;

use crate::{
    lcd_writer::LcdWriter,
    time_set::{render_time, TimeSetting},
    LCD_LINE_LENGTH,
};

const BLINK_DURATION: u16 = 20;
const LOOP_DELAY: u16 = 5;

pub enum PauseResult {
    ResumedP1,
    ResumedP2,
    Stopped,
}

pub fn pause<DP: InputPin, UP: InputPin, SP: InputPin, B: DataBus>(
    down_pin: &mut DP,
    up_pin: &mut UP,
    start_pin: &mut SP,
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
    p1_time: &TimeSetting,
    p2_time: &TimeSetting,
) -> Result<PauseResult, hd44780_driver::error::Error> {
    let mut down = debouncr::debounce_4(true);
    let mut up = debouncr::debounce_4(true);
    let mut start = debouncr::debounce_4(true);

    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, delay)?;
    render_time(p1_time, p2_time, None, writer)?;

    let mut blink_count = 0;
    Ok(loop {
        // Change blinks
        blink_count += 1;
        if blink_count >= BLINK_DURATION * 3 {
            blink_count = 0;
        }
        let blink = (blink_count / BLINK_DURATION) as u8;

        // Render
        lcd.borrow_mut().set_cursor_pos(0, delay)?;
        match blink {
            0 => uwrite!(writer, "P1  >Paused<  P2")?,
            1 => uwrite!(writer, "START to restart")?,
            _ => uwrite!(writer, "P1/P2 to resume ")?,
        }

        // Respond to input
        if start.update(
            start_pin
                .is_high()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Rising)
        {
            // Start press; reset and prompt for new time
            break PauseResult::Stopped;
        }
        if down.update(
            down_pin
                .is_high()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Rising)
        {
            // Down/P1 press; exit to P1 countdown
            break PauseResult::ResumedP1;
        }
        if up.update(up_pin.is_high().map_err(|_| hd44780_driver::error::Error)?)
            == Some(debouncr::Edge::Rising)
        {
            // Up/P2 press; exit to P2 countdown
            break PauseResult::ResumedP2;
        }

        delay_ms(LOOP_DELAY);
    })
}
