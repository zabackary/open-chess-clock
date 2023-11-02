use core::{
    cell::RefCell,
    ops::{AddAssign, SubAssign},
};

use arduino_hal::{delay_ms, Delay};
use embedded_hal::digital::v2::InputPin;
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::{derive::uDebug, uwrite};

use crate::{error::RuntimeError, lcd_writer::LcdWriter, LCD_LINE_LENGTH};

const BLINK_DURATION: u16 = 100;
const HOLD_THRESHOLD: u16 = 150;
const REPEAT_THRESHOLD: u16 = 20;
const LOOP_DELAY: u16 = 5;

#[derive(uDebug, PartialEq, Eq, Clone, Copy)]
pub enum TimeSetPart {
    P1SetMin,
    P1SetSec,
    P2SetMin,
    P2SetSec,
}

#[derive(uDebug, PartialEq, Eq, Clone, Copy)]
pub struct TimeSetting(u16);

impl TimeSetting {
    const MAX_TIME: u16 = (60 * 60 * 10) - 1;

    pub fn new(seconds: u16) -> TimeSetting {
        TimeSetting(seconds)
    }

    pub fn into_hrs_mins_secs(&self) -> (u8, u8, u8) {
        return (
            (self.0 / (60 * 60)) as u8,
            ((self.0 / 60) % 60) as u8,
            (self.0 % 60) as u8,
        );
    }

    pub fn into_millis(&self) -> u32 {
        self.0 as u32 * 1000
    }
}

impl AddAssign<u16> for TimeSetting {
    fn add_assign(&mut self, rhs: u16) {
        // If overflow or too high, go back to zero
        match self.0.checked_add(rhs) {
            Some(r) if r <= TimeSetting::MAX_TIME => self.0 = r,
            Some(_) | None => self.0 = 0,
        }
    }
}

impl SubAssign<u16> for TimeSetting {
    fn sub_assign(&mut self, rhs: u16) {
        // If underflow, wrap to highest value
        match self.0.checked_sub(rhs) {
            Some(r) => self.0 = r,
            None => self.0 = TimeSetting::MAX_TIME,
        }
    }
}

/// Prompts the user to set the time using the provided pins and LCD. Blocks.
///
/// # Usage
/// ```
/// time_set(
///   pins.d1.into_input(), // Down button
///   pins.d2.into_input(), // Up button
///   pins.d3.into_input()  // Select button
/// );
/// ```
pub fn time_set<DP: InputPin, UP: InputPin, SP: InputPin, B: DataBus>(
    down_pin: &mut DP,
    up_pin: &mut UP,
    start_pin: &mut SP,
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
) -> Result<(TimeSetting, TimeSetting), RuntimeError> {
    lcd.borrow_mut()
        .set_cursor_pos(0, delay)
        .map_err(|_| RuntimeError::LcdError)?;
    uwrite!(writer, "P1  Set time  P2").map_err(|_| RuntimeError::LcdError)?;
    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, delay)
        .map_err(|_| RuntimeError::LcdError)?;
    uwrite!(writer, "0:00:00  0:00:00").map_err(|_| RuntimeError::LcdError)?;

    let mut state = TimeSetPart::P1SetMin;
    let mut p1_setting = TimeSetting::new(0);
    let mut p2_setting = TimeSetting::new(0);

    let mut down = debouncr::debounce_4(false);
    let mut down_hold_count: u16 = 0;
    let mut up = debouncr::debounce_4(false);
    let mut up_hold_count: u16 = 0;
    let mut start = debouncr::debounce_4(false);

    let mut blink_count = 0;
    let mut last_p1_setting = TimeSetting::new(u16::MAX);
    let mut last_p2_setting = TimeSetting::new(u16::MAX);
    let mut last_blink = Some(TimeSetPart::P1SetMin);
    loop {
        // Change blinks
        blink_count += 1;
        let blink = blink_count >= BLINK_DURATION;
        if blink_count >= BLINK_DURATION * 2 {
            blink_count = 0;
        }

        // Update states
        if up.update(up_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
        {
            // Up press
            match state {
                TimeSetPart::P1SetMin => p1_setting += 60,
                TimeSetPart::P1SetSec => p1_setting += 1,
                TimeSetPart::P2SetMin => p2_setting += 60,
                TimeSetPart::P2SetSec => p2_setting += 1,
            }
            up_hold_count = 0;
            blink_count = 0;
        }
        if up_hold_count == HOLD_THRESHOLD {
            // Up hold
            match state {
                TimeSetPart::P1SetMin => p1_setting += 60,
                TimeSetPart::P1SetSec => p1_setting += 5,
                TimeSetPart::P2SetMin => p2_setting += 60,
                TimeSetPart::P2SetSec => p2_setting += 5,
            }
            blink_count = 0;
            up_hold_count += 1;
        }
        if up.is_high() {
            up_hold_count += 1;
        }
        if up_hold_count >= HOLD_THRESHOLD + REPEAT_THRESHOLD {
            up_hold_count = HOLD_THRESHOLD
        }

        if down.update(down_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
        {
            // Down press
            match state {
                TimeSetPart::P1SetMin => p1_setting -= 60,
                TimeSetPart::P1SetSec => p1_setting -= 1,
                TimeSetPart::P2SetMin => p2_setting -= 60,
                TimeSetPart::P2SetSec => p2_setting -= 1,
            }
            down_hold_count = 0;
            blink_count = 0;
        }
        if down_hold_count == HOLD_THRESHOLD {
            // Down hold
            match state {
                TimeSetPart::P1SetMin => p1_setting -= 60,
                TimeSetPart::P1SetSec => p1_setting -= 5,
                TimeSetPart::P2SetMin => p2_setting -= 60,
                TimeSetPart::P2SetSec => p2_setting -= 5,
            }
            blink_count = 0;
            down_hold_count += 1;
        }
        if down.is_high() {
            down_hold_count += 1;
        }
        if down_hold_count >= HOLD_THRESHOLD + REPEAT_THRESHOLD {
            down_hold_count = HOLD_THRESHOLD
        }

        if start.update(start_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Falling)
        {
            // Start button released; go to next portion
            state = match state {
                TimeSetPart::P1SetMin => TimeSetPart::P1SetSec,
                TimeSetPart::P1SetSec => TimeSetPart::P2SetMin,
                TimeSetPart::P2SetMin => TimeSetPart::P2SetSec,
                TimeSetPart::P2SetSec => break,
            }
        }

        // Render results
        lcd.borrow_mut()
            .set_cursor_pos(LCD_LINE_LENGTH * 1, delay)
            .map_err(|_| RuntimeError::LcdError)?;
        let new_blink = if blink { Some(state) } else { None };
        if p1_setting != last_p1_setting || p2_setting != last_p2_setting || new_blink != last_blink
        {
            render_time(&p1_setting, &p2_setting, new_blink, writer)
                .map_err(|_| RuntimeError::LcdError)?;
            last_p1_setting = p1_setting;
            last_p2_setting = p2_setting;
            last_blink = new_blink;
        } else {
            delay_ms(LOOP_DELAY);
        }
    }
    Ok((p1_setting, p2_setting))
}

pub fn render_time<B: DataBus>(
    p1_time: &TimeSetting,
    p2_time: &TimeSetting,
    blink_off_part: Option<TimeSetPart>,
    writer: &mut LcdWriter<'_, B>,
) -> Result<(), hd44780_driver::error::Error> {
    let p1_parts = p1_time.into_hrs_mins_secs();
    if !blink_off_part.is_some_and(|b| b == TimeSetPart::P1SetMin) {
        // Hour
        uwrite!(writer, "{}:", p1_parts.0.min(9))?;
        // Minute
        if p1_parts.1 > 9 {
            uwrite!(writer, "{}:", p1_parts.1)?;
        } else {
            uwrite!(writer, "0{}:", p1_parts.1)?;
        }
    } else {
        uwrite!(writer, " :  :")?;
    }
    if !blink_off_part.is_some_and(|b| b == TimeSetPart::P1SetSec) {
        // Second
        if p1_parts.2 > 9 {
            uwrite!(writer, "{}  ", p1_parts.2)?;
        } else {
            uwrite!(writer, "0{}  ", p1_parts.2)?;
        }
    } else {
        uwrite!(writer, "    ")?;
    }
    let p2_parts = p2_time.into_hrs_mins_secs();
    if !blink_off_part.is_some_and(|b| b == TimeSetPart::P2SetMin) {
        // Hour
        uwrite!(writer, "{}:", p2_parts.0.min(9))?;
        // Minute
        if p2_parts.1 > 9 {
            uwrite!(writer, "{}:", p2_parts.1)?;
        } else {
            uwrite!(writer, "0{}:", p2_parts.1)?;
        }
    } else {
        uwrite!(writer, " :  :")?;
    }
    if !blink_off_part.is_some_and(|b| b == TimeSetPart::P2SetSec) {
        // Second
        if p2_parts.2 > 9 {
            uwrite!(writer, "{}", p2_parts.2)?;
        } else {
            uwrite!(writer, "0{}", p2_parts.2)?;
        }
    } else {
        uwrite!(writer, "  ")?;
    }
    Ok(())
}
