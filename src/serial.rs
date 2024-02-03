use arduino_hal::{hal::Atmega, usart::UsartOps, Usart};
use embedded_hal::serial::{Read, Write};
use void::ResultVoidExt;

use crate::millis::millis;

/// The types of sendable messages.
/// Keep up to date with /src/serial.rs
///
/// Note: all messages are big-endian
pub enum SerialMsg {
    /// Just send HandshakeResponse back if you get this.
    ///
    /// Parameter: mode
    ///   0x0000 - let the other party decide
    ///   0x0001 - sync
    ///   0x0002 - other party is slave
    ///   0x0003 - other party is master
    ///
    /// 0xc0
    Handshake { mode: u32 },
    /// Yay, Handshake was successful!
    ///
    /// Parameter: selected_mode
    ///   0x0000 - mode not supported
    ///   0x0001 - sync
    ///   0x0002 - we're slave
    ///   0x0003 - we're master
    ///
    /// 0xc1
    HandshakeResponse { selected_mode: u32 },
    /// P1 is now counting down, and P2 has the specified # of ms.
    ///
    /// 0xc2
    StartP1 { p2_time: u32 },
    /// P2 is now counting down, and P1 has the specified # of ms.
    ///
    /// 0xc3
    StartP2 { p1_time: u32 },
    /// Syncing time. P1 first, then P2, in ms.
    ///
    /// 0xc4
    Sync { p1_time: u32, p2_time: u32 },
    /// Pause both times. The currently running clock finished at the specified
    /// ms.
    ///
    /// 0xc5
    Pause { time: u32 },
    /// P1 won by time.
    ///
    /// 0xc6
    P1Finish,
    /// P2 won by time.
    ///
    /// 0xc7
    P2Finish,
}

impl SerialMsg {
    fn to_u8(&self) -> u8 {
        match *self {
            SerialMsg::Handshake { mode: _ } => 0xc0,
            SerialMsg::HandshakeResponse { selected_mode: _ } => 0xc1,
            SerialMsg::StartP1 { p2_time: _ } => 0xc2,
            SerialMsg::StartP2 { p1_time: _ } => 0xc3,
            SerialMsg::Sync {
                p1_time: _,
                p2_time: _,
            } => 0xc4,
            SerialMsg::Pause { time: _ } => 0xc5,
            SerialMsg::P1Finish => 0xc6,
            SerialMsg::P2Finish => 0xc7,
        }
    }

    fn is_connection_message(&self) -> bool {
        match *self {
            SerialMsg::Handshake { mode: _ } => true,
            SerialMsg::HandshakeResponse { selected_mode: _ } => true,
            _ => false,
        }
    }
}

pub struct SerialHandler<USART: UsartOps<Atmega, RX, TX>, RX, TX> {
    serial: Usart<USART, RX, TX>,
    wait_start: Option<u32>,
    pub connected: bool,
}

/// Handles serial communication between the firmware and website.
/// Keep up to date with /www/src/serial.ts
impl<USART: UsartOps<Atmega, RX, TX>, RX, TX> SerialHandler<USART, RX, TX> {
    fn write_u32(&mut self, v: u32) {
        let bytes = v.to_be_bytes();
        for byte in bytes {
            nb::block!(self.serial.write(byte)).void_unwrap();
        }
    }

    pub fn write(&mut self, msg: SerialMsg) {
        // don't write to serial if not connected
        if self.connected || msg.is_connection_message() {
            self.serial.write_byte(msg.to_u8());
            match msg {
                SerialMsg::Handshake { mode } => {
                    self.write_u32(mode);
                }
                SerialMsg::HandshakeResponse { selected_mode } => {
                    self.write_u32(selected_mode);
                }
                SerialMsg::StartP1 { p2_time } => {
                    self.write_u32(p2_time);
                }
                SerialMsg::StartP2 { p1_time } => {
                    self.write_u32(p1_time);
                }
                SerialMsg::Sync { p1_time, p2_time } => {
                    self.write_u32(p1_time);
                    self.write_u32(p2_time);
                }
                SerialMsg::Pause { time } => {
                    self.write_u32(time);
                }
                SerialMsg::P1Finish => {}
                SerialMsg::P2Finish => {}
            }
        }
    }

    fn read_u32(&mut self) -> u32 {
        ((nb::block!(self.serial.read()).void_unwrap() as u32) << 24)
            | ((nb::block!(self.serial.read()).void_unwrap() as u32) << 16)
            | ((nb::block!(self.serial.read()).void_unwrap() as u32) << 8)
            | nb::block!(self.serial.read()).void_unwrap() as u32
    }

    /// Reads a raw message from the wire and blocks until it's completely received.
    fn raw_read(&mut self) -> nb::Result<SerialMsg, void::Void> {
        let msg = self.serial.read()?;
        match msg {
            0xc0 => Ok(SerialMsg::Handshake {
                mode: self.read_u32(),
            }),
            0xc1 => Ok(SerialMsg::HandshakeResponse {
                selected_mode: self.read_u32(),
            }),
            0xc2 => Ok(SerialMsg::StartP1 {
                p2_time: self.read_u32(),
            }),
            0xc3 => Ok(SerialMsg::StartP2 {
                p1_time: self.read_u32(),
            }),
            0xc4 => Ok(SerialMsg::Sync {
                p1_time: self.read_u32(),
                p2_time: self.read_u32(),
            }),
            0xc5 => Ok(SerialMsg::Pause {
                time: self.read_u32(),
            }),
            _ => {
                // Huh? Malformed message, this isn't good, ignore the message
                Err(nb::Error::WouldBlock)
            }
        }
    }

    pub fn read(&mut self) -> nb::Result<SerialMsg, void::Void> {
        let msg = self.raw_read()?;
        match msg {
            SerialMsg::Handshake { mode } => {
                self.connected = true;
                let selected_mode = match mode {
                    0x0000 => 0x0002, // if we decide, make them a slave
                    0x0001 => 0x0001, // syncing
                    0x0002 => 0x0000, // if we're a slave, it's not supported
                    0x0003 => 0x0003, // we're master
                    _ => 0x0000,      // unsupported value
                };
                self.write(SerialMsg::HandshakeResponse { selected_mode });
                Err(nb::Error::WouldBlock)
            }
            SerialMsg::HandshakeResponse { selected_mode: _ } => {
                // hopefully selected_mode is ok. we don't have the resources to check
                self.connected = true;
                Err(nb::Error::WouldBlock)
            }
            _ => Ok(msg),
        }
    }

    pub fn check_connection(&mut self, timeout_ms: u16) -> nb::Result<bool, void::Void> {
        if let None = self.wait_start {
            self.connected = false;
            self.wait_start = Some(millis());
            self.write(SerialMsg::Handshake { mode: 0x0002 });
        }
        let wait_start = self.wait_start.unwrap();
        match self.read() {
            Ok(_) => {
                // ignore. hopefully result isn't anything important.
                Err(nb::Error::WouldBlock)
            }
            Err(nb::Error::WouldBlock) => {
                if self.connected {
                    Ok(true)
                } else if (millis() - wait_start) >= timeout_ms.into() {
                    Ok(false)
                } else {
                    // ignore. still waiting.
                    Err(nb::Error::WouldBlock)
                }
            }
            Err(x) => Err(x),
        }
    }

    pub fn new(serial: Usart<USART, RX, TX>) -> Self {
        let new = Self {
            serial,
            wait_start: None,
            connected: false,
        };
        new
    }
}
