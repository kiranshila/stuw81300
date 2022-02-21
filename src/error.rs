use core::fmt;
use embedded_hal::{blocking::spi, digital::v2::OutputPin};

/// Error type throwable by vco operations
pub enum Error<SPI, LE>
where
    SPI: spi::Transfer<u8>,
    LE: OutputPin,
{
    /// Error during SPI Transfer
    Transfer(<SPI as spi::Transfer<u8>>::Error),
    /// Error during Latch Enable
    LatchEnable(<LE as OutputPin>::Error),
}

impl<SPI, LE> fmt::Debug for Error<SPI, LE>
where
    SPI: spi::Transfer<u8>,
    SPI::Error: fmt::Debug,
    LE: OutputPin,
    <LE as OutputPin>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Transfer(error) => write!(f, "Transfer({:?})", error),
            Error::LatchEnable(error) => write!(f, "LatchEnable({:?})", error),
        }
    }
}
