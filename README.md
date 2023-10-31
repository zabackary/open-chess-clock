# rust-chess-clock

A simple chess clock for the _Arduino Uno_ (specifically the r3, though it probably doesn't matter). Basically, it has an up/P1 button, a down/P2 button, and a start/stop button for configuration.

## States

State diagram
```
                      .----------------.
                      |                |
                      v                | UP/DN
┌----------┐ ST  ┌-----------┐ ST  ┌-------┐
| Time set | --> | Countdown | --> | Pause |
└----------┘     └-----------┘     └-------┘
    ^  ^               | (finish)      | ST
    |  |               |               |
    |  `---------------|---------------'
    |                  |
    |                  v
    |        ST ┌------------┐
    `---------- | Win screen |
                └------------┘
```

Time set ([`time_set.rs`](./src/time_set.rs)):
```
P1  Set time  P2
0:00:00  0:00:00
```

Pause ([`pause.rs`](./src/pause.rs)):
```
P1  >Paused<  P2
0:00:00  0:00:00
```
Top line alternates between `P1  >Paused<  P2`, `START to restart`, and `P1/P2 to resume `.

Countdown ([`countdown.rs`](./src/countdown.rs)):
```
[P1]   <<    P2 
0:00:00  0:00:00
```

Finish (*not implemented*)
```
[P1]   Time's up
0:00:00  0:00:05
```
or
```
Time's up   [P2]
0:00:05  0:00:00
```

## Hardware connections

1. LCD
   LCD RS => Arduino d7 (register select)
   LCD EN => Arduino d8 (enable)
   LCD d4 => Arduino d9
   LCD d5 => Arduino d10
   LCD d6 => Arduino d11
   LCD d7 => Arduino d12
   The LCD backlight cathode and anode should be connected, and the contrast should be set to an appropriate amount using a pot.
2. Buttons
   Down button => Arduino d2 & GND (also functions as P1 button)
   Start button => Arduino d3 & GND
   UP button => Arduino d4 & GND (also functions as P2 button)

## Build Instructions
1. Install prerequisites as described in the [`avr-hal` README] (`avr-gcc`, `avr-libc`, `avrdude`, [`ravedude`]).

2. Run `cargo build` to build the firmware.

3. Run `cargo run` to flash the firmware to a connected board.  If `ravedude`
   fails to detect your board, check its documentation at
   <https://crates.io/crates/ravedude>.

4. `ravedude` will open a console session after flashing where you can interact
   with the UART console of your board.

[`avr-hal` README]: https://github.com/Rahix/avr-hal#readme
[`ravedude`]: https://crates.io/crates/ravedude

## License
Licensed under either of

 - Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
