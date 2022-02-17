mod registers;
mod spi;

/// Enum representation of the pin 36 supply voltage
pub enum SupplyVoltage {
    /// Input voltage is 3.6 to 5.4
    LowVoltage,
    /// Input voltage is 5.0 to 5.4
    HighVoltage,
}

pub struct STuW81300<SPI, LE> {
    spi: SPI,
    le: LE,
    supply_voltage: SupplyVoltage,
    ref_freq: f32,
}

impl<SPI, LE> STuW81300<SPI, LE> {
    pub fn new(spi: SPI, le: LE, supply_voltage: SupplyVoltage, ref_freq: f32) -> Self {
        STuW81300 {
            spi,
            le,
            supply_voltage,
            ref_freq,
        }
    }
}
