///! Provides a mock SPI instance that behaves like the STuW81300
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;

enum MockPinState {
    High,
    Low,
}

pub struct MockStuw81300SPI {
    registers: [u32; 12],
}

pub struct MockStuw81300LE {
    state: MockPinState,
}

#[derive(Debug)]
pub enum MockError {}

impl MockStuw81300SPI {
    pub fn read(&self, addr: usize) -> u32 {
        self.registers[addr]
    }
    pub fn write(&mut self, addr: usize, data: u32) {
        self.registers[addr] = data
    }
}

impl Default for MockStuw81300SPI {
    fn default() -> Self {
        MockStuw81300SPI {
            registers: [
                0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0x0008052,
            ],
        }
    }
}

impl Default for MockStuw81300LE {
    fn default() -> Self {
        MockStuw81300LE {
            state: MockPinState::Low,
        }
    }
}

impl Transfer<u8> for MockStuw81300SPI {
    type Error = MockError;

    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
        let mut buf: [u8; 4] = [0; 4];
        buf[..4].clone_from_slice(&words[..4]);
        let cmd = u32::from_be_bytes(buf);
        let read = cmd >> 31 == 1;
        let addr = ((cmd >> 27) & 0b1111) as usize;
        let mut data = cmd & 0x7FFFFFF;
        if read {
            data = self.read(addr);
        } else {
            self.write(addr, data);
            data = 0u32;
        }
        let data_buf = data.to_be_bytes();
        words[..4].clone_from_slice(&data_buf[..4]);
        Ok(words)
    }
}

impl OutputPin for MockStuw81300LE {
    type Error = MockError;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.state = MockPinState::Low;
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.state = MockPinState::High;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mock() {
        let mut spi = MockStuw81300SPI::default();
        spi.transfer(&mut [0x48, 0x00, 0x00, 0x00]).unwrap();
        spi.transfer(&mut [0x40, 0x00, 0x00, 0x03]).unwrap();
        let mut read: [u8; 4] = [0xC0, 0x00, 0x00, 0x03];
        spi.transfer(&mut read).unwrap();
        assert_eq!(u32::from_be_bytes(read), 3);
    }
}
