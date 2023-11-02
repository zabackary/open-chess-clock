use core::cell::RefCell;

use arduino_hal::{delay_ms, Delay};
use embedded_hal::digital::v2::InputPin;
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uwrite;

use crate::{
    error::RuntimeError,
    lcd_writer::LcdWriter,
    time_set::{render_time, TimeSetting},
    LCD_LINE_LENGTH,
};

const BLINK_DURATION: u16 = 400;
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
    initial_pause: bool,
) -> Result<PauseResult, RuntimeError> {
    let mut down = debouncr::debounce_4(false);
    let mut up = debouncr::debounce_4(false);
    let mut start = debouncr::debounce_4(false);

    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, delay)
        .map_err(|_| RuntimeError::LcdError)?;
    render_time(p1_time, p2_time, None, writer).map_err(|_| RuntimeError::LcdError)?;

    let mut blink_count = 0;
    let mut last_blink = u8::MAX;
    Ok(loop {
        // Change blinks
        blink_count += 1;
        if blink_count >= BLINK_DURATION * 3 {
            blink_count = 0;
        }
        let blink = (blink_count / BLINK_DURATION) as u8;

        // Lazy render
        if blink != last_blink {
            lcd.borrow_mut()
                .set_cursor_pos(0, delay)
                .map_err(|_| RuntimeError::LcdError)?;
            uwrite!(
                writer,
                "{}",
                if initial_pause {
                    match blink {
                        0 => " P1/P2 to begin ",
                        1 => "START to cancel ",
                        _ => " P1          P2 ",
                    }
                } else {
                    match blink {
                        0 => " P1  Paused  P2 ",
                        1 => "START to restart",
                        _ => "P1/P2 to resume ",
                    }
                }
            )
            .map_err(|_| RuntimeError::LcdError)?;
            last_blink = blink;
        } else {
            delay_ms(LOOP_DELAY);
        }

        // Respond to input
        if start.update(start_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Falling)
        {
            // Start button released; reset and prompt for new time
            break PauseResult::Stopped;
        }
        if down.update(down_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
        {
            // Down/P1 press; exit to P1 countdown
            break PauseResult::ResumedP1;
        }
        if up.update(up_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
        {
            // Up/P2 press; exit to P2 countdown
            break PauseResult::ResumedP2;
        }
    })
}
