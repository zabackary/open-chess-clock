#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(never_type)]

use core::cell::RefCell;

use arduino_hal::{default_serial, delay_ms, hal::Atmega, usart::UsartOps, Delay};
use countdown::Turn;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use error::RuntimeError;
use hd44780_driver::{bus::DataBus, DisplayMode, HD44780};
use lcd_writer::LcdWriter;
use panic_halt as _;
use serial::{SerialHandler, SerialMsg};
use ufmt::uwrite;
use void::ResultVoidExt;

mod countdown;
mod error;
mod finish;
mod lcd_writer;
mod millis;
mod pause;
mod serial;
mod time_set;

const LCD_LINE_LENGTH: u8 = 40;
const SPLASH_DURATION: u16 = 1500;
const CONNECTION_TIMEOUT_MS: u16 = 500;
const MSG_DURATION: u16 = 1500;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Initialize peripherals
    millis::init(dp.TC0);

    let mut builtin_led = pins.d13.into_output_high();
    let down_btn = pins.d2.into_pull_up_input(); // Also P1 button
    let up_btn = pins.d4.into_pull_up_input(); // Also P2 button
    let start_btn = pins.d3.into_pull_up_input();

    let buzzer = pins.d6.into_output();

    let serial = default_serial!(dp, pins, 57600);
    let serial_handler = SerialHandler::new(serial);

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
    builtin_led.set_low();

    // The main runtime is in a wrapper to handle errors properly
    if let Err(err) = runtime(
        down_btn,
        up_btn,
        start_btn,
        buzzer,
        serial_handler,
        &mut lcd_delay,
        &lcd,
        &mut writer,
    ) {
        let _ = lcd.borrow_mut().clear(&mut lcd_delay);
        let _ = lcd.borrow_mut().set_cursor_pos(0, &mut lcd_delay);
        let _ = uwrite!(writer, "fatal error");
        let _ = lcd
            .borrow_mut()
            .set_cursor_pos(LCD_LINE_LENGTH * 1, &mut lcd_delay);
        let _ = uwrite!(writer, "{:?}", err);
    }
    // Something went very wrong; blink the LED fast.
    loop {
        builtin_led.toggle();
        delay_ms(200);
    }
}

fn runtime<
    DP: InputPin,
    UP: InputPin,
    SP: InputPin,
    BP: OutputPin,
    USART: UsartOps<Atmega, RX, TX>,
    RX,
    TX,
    B: DataBus,
>(
    mut down_btn: DP,
    mut up_btn: UP,
    mut start_btn: SP,
    mut buzzer: BP,
    mut serial_handler: SerialHandler<USART, RX, TX>,
    lcd_delay: &mut Delay,
    lcd: &RefCell<HD44780<B>>,
    writer: &mut LcdWriter<'_, B>,
) -> Result<!, RuntimeError> {
    // Show the splash screen
    lcd.borrow_mut()
        .clear(lcd_delay)
        .map_err(|_| RuntimeError::LcdError)?;
    lcd.borrow_mut()
        .set_cursor_pos(0, lcd_delay)
        .map_err(|_| RuntimeError::LcdError)?;
    uwrite!(writer, " OpenChessClock ").map_err(|_| RuntimeError::LcdError)?;
    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, lcd_delay)
        .map_err(|_| RuntimeError::LcdError)?;
    let version = env!("CARGO_PKG_VERSION");
    uwrite!(writer, "     v{}     ", version).map_err(|_| RuntimeError::LcdError)?;
    delay_ms(SPLASH_DURATION);
    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, lcd_delay)
        .map_err(|_| RuntimeError::LcdError)?;
    uwrite!(writer, "  Connecting... ").map_err(|_| RuntimeError::LcdError)?;
    let connected =
        nb::block!(serial_handler.check_connection(CONNECTION_TIMEOUT_MS)).void_unwrap();
    lcd.borrow_mut()
        .set_cursor_pos(LCD_LINE_LENGTH * 1, lcd_delay)
        .map_err(|_| RuntimeError::LcdError)?;
    uwrite!(
        writer,
        "{}",
        if connected {
            "   Connected.   "
        } else {
            " No connection. "
        }
    )
    .map_err(|_| RuntimeError::LcdError)?;
    delay_ms(MSG_DURATION);

    'main: loop {
        // Prompt the user to set up the time
        let mut times = time_set::time_set(
            &mut down_btn,
            &mut up_btn,
            &mut start_btn,
            &mut serial_handler,
            lcd_delay,
            &lcd,
            writer,
        )?;
        let mut turn = match pause::pause(
            &mut down_btn,
            &mut up_btn,
            &mut start_btn,
            lcd_delay,
            &lcd,
            writer,
            &times.0,
            &times.1,
            true,
        )? {
            pause::PauseResult::ResumedP1 => Turn::P1,
            pause::PauseResult::ResumedP2 => Turn::P2,
            pause::PauseResult::Stopped => continue 'main,
        };
        let loser = loop {
            serial_handler.write(match turn {
                Turn::P1 => SerialMsg::StartP1 {
                    p2_time: times.1.into_millis(),
                },
                Turn::P2 => SerialMsg::StartP2 {
                    p1_time: times.0.into_millis(),
                },
            });
            match countdown::countdown(
                &mut down_btn,
                &mut up_btn,
                &mut start_btn,
                &mut buzzer,
                &mut serial_handler,
                lcd_delay,
                &lcd,
                writer,
                &mut times.0,
                &mut times.1,
                &mut turn,
            )? {
                countdown::CountdownResult::FinishedP1 => break Turn::P1,
                countdown::CountdownResult::FinishedP2 => break Turn::P2,
                countdown::CountdownResult::Paused => (),
            }
            serial_handler.write(SerialMsg::Pause {
                time: match turn {
                    Turn::P1 => times.0.into_millis(),
                    Turn::P2 => times.1.into_millis(),
                },
            });
            match pause::pause(
                &mut down_btn,
                &mut up_btn,
                &mut start_btn,
                lcd_delay,
                &lcd,
                writer,
                &times.0,
                &times.1,
                false,
            )? {
                pause::PauseResult::ResumedP1 => turn = Turn::P1,
                pause::PauseResult::ResumedP2 => turn = Turn::P2,
                pause::PauseResult::Stopped => continue 'main,
            }
        };
        serial_handler.write(match loser {
            Turn::P1 => SerialMsg::P1Finish,
            Turn::P2 => SerialMsg::P2Finish,
        });
        finish::finish(
            &loser,
            &mut serial_handler,
            lcd_delay,
            &lcd,
            writer,
            &mut start_btn,
            &mut buzzer,
        )?;
    }
}
