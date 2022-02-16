use crate::registers::{Register, RegisterAddr};
use embedded_hal as hal;
use hal::blocking::spi::Transfer;
use hal::digital::v2::OutputPin;

pub struct STuW81300<SPI, LE> {
    spi: SPI,
    le: LE,
}

impl<SPI, LE> STuW81300<SPI, LE> {
    pub fn new(spi: SPI, le: LE) -> Self {
        STuW81300 { spi, le }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
enum AccessMode {
    Write = 0,
    Read = 1,
}

/// Exported SPI Mode 0 (Datasheet spec)
pub const MODE: hal::spi::Mode = hal::spi::Mode {
    polarity: hal::spi::Polarity::IdleLow,
    phase: hal::spi::Phase::CaptureOnFirstTransition,
};

impl<SPI, LE, E> STuW81300<SPI, LE>
where
    SPI: Transfer<u8, Error = E>,
    LE: OutputPin<Error = E>,
{
    fn operate(&mut self, addr: RegisterAddr, data: u32, mode: AccessMode) -> Result<u32, E> {
        // Pack data
        let mut buf = pack(addr, data, mode);
        // Perform transaction. Do we care about timing?
        self.le.set_low()?;
        self.spi.transfer(&mut buf)?;
        self.le.set_high()?;
        // Extract data
        Ok(u32::from_be_bytes(buf))
    }

    fn read(&mut self, addr: RegisterAddr) -> Result<u32, E> {
        self.operate(addr, 0, AccessMode::Read)
    }

    fn write(&mut self, addr: RegisterAddr, data: u32) -> Result<(), E> {
        self.operate(addr, data, AccessMode::Write)?;
        Ok(())
    }

    pub fn device_id(&mut self) -> Result<u32, E> {
        self.read(RegisterAddr::ST11)
    }

    pub fn init(&mut self) -> Result<(), E> {
        self.write(RegisterAddr::ST9, 0)?;
        Ok(())
    }
}

fn pack(addr: RegisterAddr, data: u32, mode: AccessMode) -> [u8; 4] {
    // Guard against data size and read-only registers
    assert!(data < (2_u32.pow(27)), "Data must be 27 bits");
    if mode == AccessMode::Write {
        assert!(!addr.read_only(), "Address is read only");
    }
    // data_bytes[0] contains the msb
    let mut buf = data.to_be_bytes();
    // Zeroth index gets sent first, MSB first order
    buf[0] |= ((mode as u8) << 7) | ((addr as u8) << 3);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock as mock;
    use mock::pin::{Mock as PinMock, State as PinState, Transaction as PinTransaction};
    use mock::spi::{Mock as SpiMock, Transaction as SpiTransaction};

    #[test]
    fn register() {
        assert_eq!(RegisterAddr::ST5 as u8, 0x05);
    }

    #[test]
    fn access_mode() {
        assert_eq!(AccessMode::Write as u8, 0);
    }

    #[test]
    fn payload() {
        assert_eq!(
            pack(RegisterAddr::ST5, 0x07FFFFFF, AccessMode::Read),
            [0xAF, 0xFF, 0xFF, 0xFF]
        );
    }

    fn spi_tester(mosi: Vec<u8>, miso: Vec<u8>) -> STuW81300<SpiMock, PinMock> {
        let spi_expectations = [SpiTransaction::transfer(mosi, miso)];

        let pin_expectations = [
            PinTransaction::set(PinState::Low),
            PinTransaction::set(PinState::High),
        ];

        let spi = SpiMock::new(&spi_expectations);
        let le = PinMock::new(&pin_expectations);

        STuW81300 { spi, le }
    }

    #[test]
    fn device_id() {
        let mut vco = spi_tester(vec![0xd8, 0, 0, 0], vec![0, 0, 0x80, 0x52]);
        assert_eq!(vco.device_id().unwrap(), 0x8052);
    }

    #[test]
    fn init() {
        let mut vco = spi_tester(vec![72, 0, 0, 0], vec![0, 0, 0, 0]);
        vco.init().unwrap();
    }
}
