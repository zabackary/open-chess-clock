use core::cell::RefCell;

use arduino_hal::{delay_ms, Delay};
use embedded_hal::digital::v2::InputPin;
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uwrite;

use crate::{
    lcd_writer::LcdWriter,
    millis::millis,
    time_set::{render_time, TimeSetting},
    LCD_LINE_LENGTH,
};

const LOOP_DELAY: u16 = 5;

pub enum CountdownResult {
    FinishedP1,
    FinishedP2,
    Paused,
}

#[derive(PartialEq, Eq, Clone)]
pub enum Turn {
    P1,
    P2,
}

pub fn countdown<DP: InputPin, UP: InputPin, SP: InputPin, B: DataBus>(
    down_pin: &mut DP,
    up_pin: &mut UP,
    start_pin: &mut SP,
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
    p1_time: &mut TimeSetting,
    p2_time: &mut TimeSetting,
    turn: &mut Turn,
) -> Result<CountdownResult, hd44780_driver::error::Error> {
    let mut down = debouncr::debounce_4(false);
    let mut up = debouncr::debounce_4(false);
    let mut start = debouncr::debounce_4(false);

    // Initialize last_* variable with bogus values to prompt immediate render
    let mut last_p1_time = TimeSetting::new(u16::MAX);
    let mut p1_ms = p1_time.into_millis();
    let mut last_p2_time = TimeSetting::new(u16::MAX);
    let mut p2_ms = p2_time.into_millis();
    let mut last_frame_ms = millis();
    let mut last_turn = turn.clone();
    Ok(loop {
        let new_p1_time = convert_time(p1_ms);
        let new_p2_time = convert_time(p2_ms);
        // Lazy render
        if *turn != last_turn || new_p1_time != last_p1_time || new_p2_time != last_p2_time {
            last_turn = turn.clone();
            render(delay, lcd, &new_p1_time, &new_p2_time, turn, writer)?;
            last_p1_time = new_p1_time;
            last_p2_time = new_p2_time;
        } else {
            delay_ms(LOOP_DELAY);
        }

        let current_time = millis();
        // Compute the time difference, wrapping since u32 ms time is only ~1hr
        let difference = current_time.wrapping_sub(last_frame_ms);
        last_frame_ms = current_time;

        match turn {
            Turn::P1 => match p1_ms.checked_sub(difference) {
                Some(x) => p1_ms = x,
                None => {
                    p1_ms = 0;
                    break finish_countdown(p1_ms, p2_ms, p1_time, p2_time);
                }
            },
            Turn::P2 => match p2_ms.checked_sub(difference) {
                Some(x) => p2_ms = x,
                None => {
                    p2_ms = 0;
                    break finish_countdown(p1_ms, p2_ms, p1_time, p2_time);
                }
            },
        }

        if start.update(
            start_pin
                .is_low()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Falling)
        {
            // Start button released; pause the game
            break finish_countdown(p1_ms, p2_ms, p1_time, p2_time);
        }
        if down.update(
            down_pin
                .is_low()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Rising)
            && *turn == Turn::P2
        {
            // Down/P1 press
            *turn = Turn::P1;
        }
        if up.update(up_pin.is_low().map_err(|_| hd44780_driver::error::Error)?)
            == Some(debouncr::Edge::Rising)
            && *turn == Turn::P1
        {
            // Up/P2 press
            *turn = Turn::P2
        }
    })
}

fn render<B: DataBus>(
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    p1_time: &TimeSetting,
    p2_time: &TimeSetting,
    turn: &Turn,
    writer: &mut LcdWriter<'_, B>,
) -> Result<(), hd44780_driver::error::Error> {
    lcd.borrow_mut().set_cursor_pos(0, delay)?;
    if *turn == Turn::P1 {
        uwrite!(writer, "[P1]   <<    P2 ")?;
    } else {
        uwrite!(writer, " P1    >>   [P2]")?;
    }
    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, delay)?;
    render_time(p1_time, p2_time, None, writer)?;
    Ok(())
}

fn finish_countdown(
    p1_ms: u32,
    p2_ms: u32,
    p1_time: &mut TimeSetting,
    p2_time: &mut TimeSetting,
) -> CountdownResult {
    // Convert times back to seconds
    // TODO: round up?
    *p1_time = convert_time(p1_ms);
    *p2_time = convert_time(p2_ms);
    match (p1_ms, p2_ms) {
        (0, _) => CountdownResult::FinishedP1,
        (_, 0) => CountdownResult::FinishedP2,
        _ => CountdownResult::Paused,
    }
}

fn convert_time(x: u32) -> TimeSetting {
    TimeSetting::new((x / 1000) as u16)
}
