#![cfg_attr(not(test), no_std)]

///! This is an `embedded-hal` crate for the (STuW81300)[https://www.st.com/en/wireless-connectivity/stuw81300.html] integrated VCO/PLL chip.
///! Eventually I hope to cover the entire capabilities of the chip as a nice example of a complicated embedded-hal SPI driver.
///!
///! # Not yet implemented
///! * Charge pump leakage current
///! * Down-split current
///! * RF2 Output
mod api;
mod mock;
mod registers;
mod spi;

/// Enum representation of the pin 36 supply voltage
#[derive(Debug, PartialEq)]
pub enum SupplyVoltage {
    /// Input voltage is 3.6 to 5.4
    LowVoltage,
    /// Input voltage is 5.0 to 5.4
    HighVoltage,
}

/// The connection type of the reference clock
#[repr(u32)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ReferenceType {
    /// Ref clock is connected to pin 21
    SingleEnded = 3,
    /// Reference clock is differential between 20 and 21
    Differential = 1,
    /// Crystal oscillator connected between 20 and 21
    Crystal = 2,
}

pub struct STuW81300<SPI, LE> {
    spi: SPI,
    le: LE,
    supply_voltage: SupplyVoltage,
    ref_freq: f32,
    ref_type: ReferenceType,
}

impl<SPI, LE> STuW81300<SPI, LE> {
    pub fn new(
        spi: SPI,
        le: LE,
        supply_voltage: SupplyVoltage,
        ref_freq: f32,
        ref_type: ReferenceType,
    ) -> Self {
        assert!(
            (10e6..=800e6).contains(&ref_freq),
            "Reference frequency out of range"
        );
        STuW81300 {
            spi,
            le,
            supply_voltage,
            ref_freq,
            ref_type,
        }
    }
}
