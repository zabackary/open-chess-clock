#![no_std]
#![no_main]

use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer0Pwm, Timer2Pwm};
use panic_halt as _;
use ufmt::uwriteln;

const DELAY_TIME: u16 = 10;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    let timer0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);

    let mut red = pins.d6.into_output().into_pwm(&timer0);
    red.enable();
    let mut green = pins.d5.into_output().into_pwm(&timer0);
    green.enable();
    let mut blue = pins.d3.into_output().into_pwm(&timer2);
    blue.enable();
    let mut counter: isize = 0;

    loop {
        if counter < 767 {
            counter += 1;
        } else {
            counter = 0;
        }
        let (r, g, b) = get_intensities(counter);
        red.set_duty(r / 5);
        green.set_duty(g / 5);
        blue.set_duty(b / 5);
        arduino_hal::delay_ms(DELAY_TIME);
    }
}

/// From https://github.com/mclarkk/arduino-rgb-tutorial/blob/master/rainbow_fade.ino
fn get_intensities(color: isize) -> (u8, u8, u8) {
    if color <= 255 {
        ((255 - color) as u8, color as u8, 0)
    } else if color <= 511 {
        (0, (255 - (color - 256)) as u8, (color - 256) as u8)
    } else {
        ((color - 512) as u8, 0, (255 - (color - 512)) as u8)
    }
}
