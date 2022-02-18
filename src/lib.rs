///! # Not yet implemented
///! ## Fractional Spurs and Compensation Control
///! * PFD delay mode
///! * Charge pump leakage current
///! * Down-split current
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
