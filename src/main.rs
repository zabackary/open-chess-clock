#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::cell::RefCell;

use arduino_hal::{delay_ms, prelude::_void_ResultVoidExt, Delay};
use countdown::Turn;
use hd44780_driver::{DisplayMode, HD44780};
use panic_halt as _;
use ufmt::{uwrite, uwriteln};

mod countdown;
mod lcd_writer;
mod millis;
mod pause;
mod time_set;

const LCD_LINE_LENGTH: u8 = 40;
const SPLASH_DURATION: u16 = 1500;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    uwriteln!(serial, "Initializing chess clock...").void_unwrap();

    // Initialize peripherals
    let mut builtin_led = pins.d13.into_output_high();
    let mut down_btn = pins.d2.into_pull_up_input(); // Also P1 button
    let mut up_btn = pins.d4.into_pull_up_input(); // Also P2 button
    let mut start_btn = pins.d3.into_pull_up_input();
    millis::init(dp.TC0);
    let mut lcd_delay = Delay::new();
    let lcd_d4 = pins.d9.into_output();
    let lcd_d5 = pins.d10.into_output();
    let lcd_d6 = pins.d11.into_output();
    let lcd_d7 = pins.d12.into_output();
    let lcd = RefCell::new(
        HD44780::new_4bit(
            pins.d7.into_output(),
            pins.d8.into_output(),
            lcd_d4,
            lcd_d5,
            lcd_d6,
            lcd_d7,
            &mut lcd_delay,
        )
        .unwrap(),
    );
    lcd.borrow_mut()
        .set_display_mode(
            DisplayMode {
                cursor_blink: hd44780_driver::CursorBlink::Off,
                cursor_visibility: hd44780_driver::Cursor::Invisible,
                display: hd44780_driver::Display::On,
            },
            &mut lcd_delay,
        )
        .unwrap();
    let mut writer = lcd_writer::LcdWriter::new(&lcd);

    // Enable interrupts! Whoo! Things can break!
    unsafe { avr_device::interrupt::enable() };

    // Turn off the init light to show the successful end of initialization
    uwriteln!(serial, "Successfully initialized").void_unwrap();
    builtin_led.set_low();

    // Show the splash screen
    lcd.borrow_mut().clear(&mut lcd_delay).unwrap();
    lcd.borrow_mut().set_cursor_pos(0, &mut lcd_delay).unwrap();
    uwrite!(writer, " OpenChessClock ").unwrap();
    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, &mut lcd_delay)
        .unwrap();
    uwrite!(writer, "      v0.1      ").unwrap();
    delay_ms(SPLASH_DURATION);

    'main: loop {
        // Prompt the user to set up the time
        let mut times = time_set::time_set(
            &mut down_btn,
            &mut up_btn,
            &mut start_btn,
            &mut lcd_delay,
            &lcd,
            &mut writer,
        )
        .unwrap();
        uwrite!(serial, "New times - p1: {:?}, p2: {:?}", times.0, times.1).void_unwrap();
        let mut turn = countdown::Turn::P1;
        loop {
            match countdown::countdown(
                &mut down_btn,
                &mut up_btn,
                &mut start_btn,
                &mut lcd_delay,
                &lcd,
                &mut writer,
                &mut times.0,
                &mut times.1,
                &mut turn,
            )
            .unwrap()
            {
                countdown::CountdownResult::FinishedP1 => break,
                countdown::CountdownResult::FinishedP2 => break,
                countdown::CountdownResult::Paused => (),
            }
            match pause::pause(
                &mut down_btn,
                &mut up_btn,
                &mut start_btn,
                &mut lcd_delay,
                &lcd,
                &mut writer,
                &times.0,
                &times.1,
            )
            .unwrap()
            {
                pause::PauseResult::ResumedP1 => turn = Turn::P1,
                pause::PauseResult::ResumedP2 => turn = Turn::P2,
                pause::PauseResult::Stopped => continue 'main,
            }
        }

        // Out of time, need to check

        lcd.borrow_mut().set_cursor_pos(0, &mut lcd_delay).unwrap();
        uwrite!(writer, "P1            P2").unwrap();
        lcd.borrow_mut()
            .set_cursor_pos(LCD_LINE_LENGTH * 1, &mut lcd_delay)
            .unwrap();
        uwrite!(writer, "{}", millis::millis() / 1000).unwrap();
    }
}
