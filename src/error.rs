#[derive(ufmt::derive::uDebug)]
pub enum RuntimeError {
    LcdError,
    PinReadError,
    PinWriteError,
}
