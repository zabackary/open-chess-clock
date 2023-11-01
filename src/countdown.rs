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
    let mut p1_ms_at_change = p1_time.into_millis();
    let mut last_p2_time = TimeSetting::new(u16::MAX);
    let mut p2_ms_at_change = p2_time.into_millis();
    let mut last_turn = turn.clone();

    let mut last_change_time = millis();
    Ok(loop {
        let time_since_change = millis() - last_change_time;
        let new_p1_time = if *turn == Turn::P1 {
            convert_time(match p1_ms_at_change.checked_sub(time_since_change) {
                Some(x) => x,
                None => {
                    p1_ms_at_change = 0;
                    break finish_countdown(p1_ms_at_change, p2_ms_at_change, p1_time, p2_time);
                }
            })
        } else {
            convert_time(p1_ms_at_change)
        };
        let new_p2_time = if *turn == Turn::P2 {
            convert_time(match p2_ms_at_change.checked_sub(time_since_change) {
                Some(x) => x,
                None => {
                    p2_ms_at_change = 0;
                    break finish_countdown(p1_ms_at_change, p2_ms_at_change, p1_time, p2_time);
                }
            })
        } else {
            convert_time(p2_ms_at_change)
        };
        // Lazy render
        if *turn != last_turn || new_p1_time != last_p1_time || new_p2_time != last_p2_time {
            last_turn = turn.clone();
            render(delay, lcd, &new_p1_time, &new_p2_time, turn, writer)?;
            last_p1_time = new_p1_time;
            last_p2_time = new_p2_time;
        } else {
            delay_ms(LOOP_DELAY);
        }

        if start.update(
            start_pin
                .is_low()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Falling)
        {
            // Unsafe subtraction since it's already been checked in the rendering code
            match *turn {
                Turn::P1 => {
                    p1_ms_at_change = p1_ms_at_change - time_since_change;
                }
                Turn::P2 => {
                    p2_ms_at_change = p2_ms_at_change - time_since_change;
                }
            }
            // Start button released; pause the game
            break finish_countdown(p1_ms_at_change, p2_ms_at_change, p1_time, p2_time);
        }
        if down.update(
            down_pin
                .is_low()
                .map_err(|_| hd44780_driver::error::Error)?,
        ) == Some(debouncr::Edge::Rising)
            && *turn == Turn::P2
        {
            // Down/P1 press (switch to P2)
            // Unsafe subtraction since it's already been checked in the rendering code
            p1_ms_at_change = p1_ms_at_change - time_since_change;
            last_change_time = millis();
            *turn = Turn::P2
        }
        if up.update(up_pin.is_low().map_err(|_| hd44780_driver::error::Error)?)
            == Some(debouncr::Edge::Rising)
            && *turn == Turn::P1
        {
            // Up/P2 press (switch to P1)
            // Unsafe subtraction since it's already been checked in the rendering code
            p2_ms_at_change = p2_ms_at_change - time_since_change;
            last_change_time = millis();
            *turn = Turn::P1;
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
