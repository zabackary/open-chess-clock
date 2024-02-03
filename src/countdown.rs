use core::cell::RefCell;

use arduino_hal::{delay_ms, hal::Atmega, usart::UsartOps, Delay};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use hd44780_driver::{bus::DataBus, HD44780};
use ufmt::uwrite;

use crate::{
    error::RuntimeError,
    lcd_writer::LcdWriter,
    millis::millis,
    serial::{SerialHandler, SerialMsg},
    time_set::{render_time, TimeSetting},
    LCD_LINE_LENGTH,
};

const LOOP_DELAY: u16 = 5;
const BUZZER_LENGTH: u16 = 20;

pub enum CountdownResult {
    FinishedP1,
    FinishedP2,
    Paused,
}

#[derive(PartialEq, Eq, Clone, ufmt::derive::uDebug)]
pub enum Turn {
    P1,
    P2,
}

pub fn countdown<
    DP: InputPin,
    UP: InputPin,
    SP: InputPin,
    BP: OutputPin,
    B: DataBus,
    USART: UsartOps<Atmega, RX, TX>,
    RX,
    TX,
>(
    down_pin: &mut DP,
    up_pin: &mut UP,
    start_pin: &mut SP,
    buzzer_pin: &mut BP,
    serial_handler: &mut SerialHandler<USART, RX, TX>,
    delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
    p1_time: &mut TimeSetting,
    p2_time: &mut TimeSetting,
    turn: &mut Turn,
) -> Result<CountdownResult, RuntimeError> {
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
    let mut remaining_buzzer_duration = 0;
    Ok(loop {
        let time_since_change = millis() - last_change_time;
        let new_p1_ms = if *turn == Turn::P1 {
            match p1_ms_at_change.checked_sub(time_since_change) {
                Some(x) => x,
                None => {
                    p1_ms_at_change = 0;
                    break finish_countdown(p1_ms_at_change, p2_ms_at_change, p1_time, p2_time);
                }
            }
        } else {
            p1_ms_at_change
        };
        let new_p1_time = convert_time(new_p1_ms);
        let new_p2_ms = if *turn == Turn::P2 {
            match p2_ms_at_change.checked_sub(time_since_change) {
                Some(x) => x,
                None => {
                    p2_ms_at_change = 0;
                    break finish_countdown(p1_ms_at_change, p2_ms_at_change, p1_time, p2_time);
                }
            }
        } else {
            p2_ms_at_change
        };
        let new_p2_time = convert_time(new_p2_ms);

        // Update the buzzer
        if remaining_buzzer_duration == 1 {
            buzzer_pin
                .set_low()
                .map_err(|_| RuntimeError::PinWriteError)?;
        }
        if remaining_buzzer_duration > 0 {
            remaining_buzzer_duration -= 1;
        }
        if match *turn {
            Turn::P1 => new_p1_ms <= 1000 * 10 && new_p1_ms % 1000 == 0,
            Turn::P2 => new_p2_ms <= 1000 * 10 && new_p2_ms % 1000 == 0,
        } {
            buzzer_pin
                .set_high()
                .map_err(|_| RuntimeError::PinWriteError)?;
            remaining_buzzer_duration = BUZZER_LENGTH;
        }

        // Lazy render
        if *turn != last_turn || new_p1_time != last_p1_time || new_p2_time != last_p2_time {
            last_turn = turn.clone();
            render(delay, lcd, &new_p1_time, &new_p2_time, turn, writer)
                .map_err(|_| RuntimeError::LcdError)?;
            last_p1_time = new_p1_time;
            last_p2_time = new_p2_time;
        } else {
            delay_ms(LOOP_DELAY);
        }

        let _msg = serial_handler.read().ok();

        if start.update(start_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Falling)
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
        if down.update(down_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
            && *turn == Turn::P1
        {
            // Down/P1 press (switch to P2)
            // Unsafe subtraction since it's already been checked in the rendering code
            p1_ms_at_change = p1_ms_at_change - time_since_change;
            let _ = serial_handler.write(SerialMsg::StartP2 {
                p1_time: p1_ms_at_change, // TODO: fix this
            });
            last_change_time = millis();
            *turn = Turn::P2
        }
        if up.update(up_pin.is_low().map_err(|_| RuntimeError::PinReadError)?)
            == Some(debouncr::Edge::Rising)
            && *turn == Turn::P2
        {
            // Up/P2 press (switch to P1)
            // Unsafe subtraction since it's already been checked in the rendering code
            p2_ms_at_change = p2_ms_at_change - time_since_change;
            let _ = serial_handler.write(SerialMsg::StartP1 {
                p2_time: p2_ms_at_change, // TODO: fix this
            });
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
