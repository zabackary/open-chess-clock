use arduino_hal::{hal::Atmega, usart::UsartOps, Usart};
use embedded_hal::serial::{Read, Write};

use crate::millis::millis;

/// Note: all arguments are big-endian
pub enum SerialMsg {
    /// Just send HandshakeResponse back if you get this.
    ///
    /// 0x00
    Handshake,
    /// Yay, Handshake was successful!
    ///
    /// 0x01
    HandshakeResponse,
    /// P1 is now counting down, and P2 has the specified # of ms. This will
    /// wrap after about an hour.
    ///
    /// 0x02
    StartP1 { p2_time: u16 },
    /// P2 is now counting down, and P1 has the specified # of ms. This will
    /// wrap after about an hour.
    ///
    /// 0x03
    StartP2 { p1_time: u16 },
    /// Syncing time. P1 first, then P2, in ms.
    ///
    /// 0x04
    Sync { p1_time: u16, p2_time: u16 },
    /// Pause both times. The currently running clock finished at the specified
    /// ms.
    ///
    /// 0x05
    Pause { time: u16 },
}

impl SerialMsg {
    fn to_u8(&self) -> u8 {
        match *self {
            SerialMsg::Handshake => 0x00,
            SerialMsg::HandshakeResponse => 0x01,
            SerialMsg::StartP1 { p2_time: _ } => 0x02,
            SerialMsg::StartP2 { p1_time: _ } => 0x03,
            SerialMsg::Sync {
                p1_time: _,
                p2_time: _,
            } => 0x04,
            SerialMsg::Pause { time: _ } => 0x05,
        }
    }
}

pub struct SerialHandler<USART: UsartOps<Atmega, RX, TX>, RX, TX> {
    serial: Usart<USART, RX, TX>,
    wait_start: Option<u32>,
    pub connected: bool,
}

impl<USART: UsartOps<Atmega, RX, TX>, RX, TX> SerialHandler<USART, RX, TX> {
    fn write_u16(&mut self, v: u16) -> nb::Result<(), void::Void> {
        let bytes = v.to_be_bytes();
        self.serial.write(bytes[0])?;
        self.serial.write(bytes[1])?;
        Ok(())
    }

    pub fn write(&mut self, msg: SerialMsg) -> nb::Result<(), void::Void> {
        self.serial.write(msg.to_u8())?;
        match msg {
            SerialMsg::Handshake => {}
            SerialMsg::HandshakeResponse => {}
            SerialMsg::StartP1 { p2_time } => {
                self.write_u16(p2_time)?;
            }
            SerialMsg::StartP2 { p1_time } => {
                self.write_u16(p1_time)?;
            }
            SerialMsg::Sync { p1_time, p2_time } => {
                self.write_u16(p1_time)?;
                self.write_u16(p2_time)?;
            }
            SerialMsg::Pause { time } => {
                self.write_u16(time)?;
            }
        }
        Ok(())
    }

    /// Reads a raw message from the wire and blocks until it's completely received.
    fn raw_read(&mut self) -> nb::Result<SerialMsg, void::Void> {
        let msg = self.serial.read()?;
        match msg {
            0x00 => Ok(SerialMsg::Handshake),
            0x01 => Ok(SerialMsg::HandshakeResponse),
            0x02 => Ok(SerialMsg::StartP1 {
                p2_time: ((nb::block!(self.serial.read())? as u16) << 8)
                    | nb::block!(self.serial.read())? as u16,
            }),
            0x03 => Ok(SerialMsg::StartP2 {
                p1_time: ((nb::block!(self.serial.read())? as u16) << 8)
                    | nb::block!(self.serial.read())? as u16,
            }),
            0x04 => Ok(SerialMsg::Sync {
                p1_time: ((nb::block!(self.serial.read())? as u16) << 8)
                    | nb::block!(self.serial.read())? as u16,
                p2_time: ((nb::block!(self.serial.read())? as u16) << 8)
                    | nb::block!(self.serial.read())? as u16,
            }),
            0x05 => Ok(SerialMsg::Pause {
                time: ((nb::block!(self.serial.read())? as u16) << 8)
                    | nb::block!(self.serial.read())? as u16,
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
            SerialMsg::Handshake => {
                self.connected = true;
                self.write(SerialMsg::HandshakeResponse)?;
                Err(nb::Error::WouldBlock)
            }
            SerialMsg::HandshakeResponse => {
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
            self.write(SerialMsg::Handshake)?;
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
