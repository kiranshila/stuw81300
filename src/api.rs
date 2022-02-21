use crate::error::Error;
use crate::registers as regs;
use crate::STuW81300;
use embedded_hal as hal;
use hal::blocking::spi::Transfer;
use hal::digital::v2::OutputPin;
#[allow(unused_imports)]
use micromath::F32Ext;
use regs::RegisterAddr;

const MAX_MOD: u32 = 2097151;

// Public Enums
#[repr(u32)]
#[derive(PartialEq)]
pub enum ReferenceClockPath {
    Direct,
    Doubled,
    Halved,
    Quartered,
}

#[repr(u32)]
pub enum DsmOrder {
    ThirdOrder,
    SecondOrder,
    FirstOrder,
    FourthOrder,
}

#[repr(u8)]
pub enum PllPath {
    Direct,
    Halved,
}

#[repr(u32)]
pub enum PfdDelayMode {
    NoDelay,
    VcoDivDelay,
    RefDivDelay,
}

#[repr(u32)]
pub enum PfdDelay {
    /// 1.2 ns / 0 A
    Default,
    /// 1.9 ns / 0.25*Icp
    OneNine,
    /// 2.5 ns / 0.5*Icp
    TwoFive,
    /// 3.0 ns / 0.75*Icp
    ThreeZero,
}

impl<SPI, LE> STuW81300<SPI, LE>
where
    SPI: Transfer<u8>,
    LE: OutputPin,
{
    /// Retrieves the device id, either 0x804B or 0x8052
    pub fn device_id(&mut self) -> Result<u32, Error<SPI, LE>> {
        self.read(RegisterAddr::ST11)
    }

    /// Initializes the device
    pub fn init(&mut self) -> Result<(), Error<SPI, LE>> {
        // Initialization register
        self.write(RegisterAddr::ST9, 0)?;
        // Read device_id
        let device_id = self.device_id()?;

        // Power settings
        let mut st4: regs::ST4 = self.read_reg()?;
        match device_id {
            // STUW81300-1T and STUW81300-1TR
            0x804B => {
                st4.calb_3v3_mode1 = false;
                st4.calb_3v3_mode0 = false;
            }
            // STUW81300T and STUW81300TR
            0x8052 => {
                st4.calb_3v3_mode1 = self.supply_voltage == crate::SupplyVoltage::LowVoltage;
                st4.calb_3v3_mode0 = self.supply_voltage == crate::SupplyVoltage::LowVoltage;
            }
            _ => unreachable!(),
        }
        st4.rf_out_3v3 = self.supply_voltage == crate::SupplyVoltage::LowVoltage;
        st4.ref_buff_mode = self.ref_type as u32;

        self.write_reg(&st4)?;
        Ok(())
    }

    /// Sets the reference clock path
    /// This setting in combination with `set_reference_clock_divider` controls the frequency
    /// of the PFD. The result of which can be found with `get_pfd_frequency`
    pub fn set_reference_clock_path(
        &mut self,
        path: ReferenceClockPath,
    ) -> Result<(), Error<SPI, LE>> {
        if (self.ref_freq >= 400e6) && (self.ref_freq <= 800e6) {
            assert!(
                matches!(path, ReferenceClockPath::Quartered),
                "Reference clock path must be Quartered for reference clocks higher than 400 MHz"
            );
        } else if (self.ref_freq >= 200e6) && (self.ref_freq <= 400e6) {
            assert!(matches!(
                path,
                ReferenceClockPath::Halved | ReferenceClockPath::Quartered
            ),"Reference clock path must be Halved or Quartered for reference clocks between 200 and 400 MHz");
        } else if (self.ref_freq >= 25e6) && (self.ref_freq <= 200e6) {
            assert!(matches!(
                path,
                ReferenceClockPath::Halved | ReferenceClockPath::Quartered | ReferenceClockPath::Direct
            ),"Reference clock path cannot be doubled if the reference clock is higher than 25 MHz");
        }
        if self.ref_type == crate::ReferenceType::Differential {
            assert!(
                path != ReferenceClockPath::Doubled,
                "Reference clock path of doubled is not applicable in differential mode"
            );
        }

        let mut st3: regs::ST3 = self.read_reg()?;
        st3.ref_path_sel = path as u32;
        self.write_reg(&st3)?;
        Ok(())
    }

    /// Sets the reference clock divider for the PFD. This must be between 1 and 8191.
    pub fn set_reference_clock_divider(&mut self, r: u32) -> Result<(), Error<SPI, LE>> {
        assert!(
            (1..=8191).contains(&r),
            "The reference clock divider ratio must be between 1 and 8191"
        );
        let mut st3: regs::ST3 = self.read_reg()?;
        st3.r = r;
        self.write_reg(&st3)
    }

    /// Gets the internal phase-frequency detector (PFD) frequency in Hz
    pub fn get_pfd_frequency(&mut self) -> Result<f32, Error<SPI, LE>> {
        let st3: regs::ST3 = self.read_reg()?;
        let r = st3.r as f32;
        let first_stage = match st3.ref_path_sel {
            0 => self.ref_freq,
            1 => self.ref_freq * 2f32,
            2 => self.ref_freq / 2f32,
            3 => self.ref_freq / 4f32,
            _ => unreachable!(),
        };
        Ok(first_stage / r)
    }

    /// Gets the current output frequency in Hz
    pub fn get_output_frequency(&mut self) -> Result<f32, Error<SPI, LE>> {
        // Grab all the registers we need to calculate this
        let st0: regs::ST0 = self.read_reg()?;
        let st1: regs::ST1 = self.read_reg()?;
        let st2: regs::ST2 = self.read_reg()?;
        let st6: regs::ST6 = self.read_reg()?;
        // Depending if the output is > 6 GHz (in which case PLL_SEL will be set high)
        // this is doubled
        let n_int = st0.n as f32;
        let frac = st1.frac as f32;
        let modu = st2.modu as f32;
        let dithering = (st6.dithering as u32) as f32;
        let n = n_int + frac / modu + dithering / (2f32 * modu);
        let f_out = self.get_pfd_frequency()? * n as f32;
        if st1.pll_sel {
            Ok(2f32 * f_out)
        } else {
            Ok(f_out)
        }
    }

    /// Set the dithering function, used to reduce the fractional spur tones by
    /// spreading the DSM sequence and consequently the energy of the spurs over
    /// a wider bandwidth
    pub fn set_dithering(&mut self, active: bool) -> Result<(), Error<SPI, LE>> {
        let mut st6: regs::ST6 = self.read_reg()?;
        st6.dithering = active;
        self.write_reg(&st6)
    }

    /// Sets the delta-sigma modulator order. Only has an effect when the divider ratio has
    /// fractional components. It is recommended to use the `ThirdOrder` setting.
    pub fn set_dsm_order(&mut self, order: DsmOrder) -> Result<(), Error<SPI, LE>> {
        let mut st6: regs::ST6 = self.read_reg()?;
        st6.dsm_order = order as u32;
        self.write_reg(&st6)
    }

    /// Sets the MOD value for Fractional-N operation
    pub fn set_mod(&mut self, modu: u32) -> Result<(), Error<SPI, LE>> {
        assert!(
            (2..=MAX_MOD).contains(&modu),
            "MOD must be between 2 and 2097151"
        );
        let mut st2: regs::ST2 = self.read_reg()?;
        st2.modu = modu;
        self.write_reg(&st2)
    }

    /// Sets the FRAC value for Fractional-N operation, MOD must be set first
    pub fn set_frac(&mut self, frac: u32) -> Result<(), Error<SPI, LE>> {
        let st2: regs::ST2 = self.read_reg()?;
        assert!(
            frac <= st2.modu,
            "FRAC must be between 0 and MOD-1, set MOD first",
        );
        let mut st1: regs::ST1 = self.read_reg()?;
        st1.frac = frac;
        self.write_reg(&st1)
    }

    /// Sets the divider ratio, maximizing MOD to reduce frequency error
    /// Also, the calibrator frequency is set accordingly to the maximum of 250 kHz
    pub fn set_divider_ratio(&mut self, n: f32) -> Result<(), Error<SPI, LE>> {
        assert!(n >= 24f32, "Division ratio must be greater than 23");
        // Valid divider ratios are controlled by the DSM, if there is a fraction part
        let n_int = n.trunc();
        let n_frac = n.fract();
        if n_int >= 512f32 {
            assert!(
                n_frac == 0f32,
                "Division ratios larger than 512 can't have fractional components"
            );
        }
        let st6: regs::ST6 = self.read_reg()?;
        let mut st0: regs::ST0 = self.read_reg()?;
        let mut st1: regs::ST1 = self.read_reg()?;
        let mut st2: regs::ST2 = self.read_reg()?;

        match st6.dsm_order {
            0 => assert!(
                (27f32..=507f32).contains(&n),
                "Third order DSM requires 27 <= N <= 507"
            ), // Third Order
            1 => assert!(
                (25f32..=509f32).contains(&n),
                "Second order DSM requires 25 <= N <= 509"
            ), // Second Order
            2 => assert!(
                (24f32..=510f32).contains(&n),
                "First order DSM requires 24 <= N <= 510"
            ), // First Order
            3 => assert!(
                (31f32..=503f32).contains(&n),
                "Third order DSM requires 31 <= N <= 503"
            ), // Fourth Order
            _ => unreachable!(),
        };

        let modu = MAX_MOD;
        let frac = (((2f32 * n_frac * (modu as f32)) - ((st6.dithering as u32) as f32)) / 2f32)
            .round() as u32;

        st0.n = n_int as u32;
        st1.frac = frac;
        st2.modu = modu;

        self.write_reg(&st0)?;
        self.write_reg(&st1)?;
        self.write_reg(&st2)
    }

    /// Sets the signal path to the PLL. This must be `Halved` for VCO operation above 6 GHz.
    pub fn set_pll_path(&mut self, path: PllPath) -> Result<(), Error<SPI, LE>> {
        let mut st1: regs::ST1 = self.read_reg()?;
        st1.pll_sel = path as u8 == 1;
        self.write_reg(&st1)
    }

    /// Gets the signal path to the PLL
    pub fn get_pll_path(&mut self) -> Result<PllPath, Error<SPI, LE>> {
        let st1: regs::ST1 = self.read_reg()?;
        Ok(match st1.pll_sel {
            true => PllPath::Direct,
            false => PllPath::Halved,
        })
    }

    /// Sets the desired output frequency
    /// Only RF1 (3-8 GHz) at the moment
    /// There are an infinite number of solutions for the various configurations in this device,
    /// so the strategy here is to minimize spurs. It does this by maximizing FRAC and MOD, keeping the
    /// same FRAC/MOD ratio and setting DITHERING to 1. As a drawback, there will be small frequency error.
    /// Also, the calibrator frequency is set accordingly to the maximum of 250 kHz
    ///
    /// This function may fail if the computed divider ratio isn't feasable, in which case changes to the DSM order
    /// and reference divider network may be necessary
    pub fn set_output_frequency(&mut self, f: f32) -> Result<(), Error<SPI, LE>> {
        self.set_dithering(true)?;
        let fpfd = self.get_pfd_frequency()?;
        let mut n = f / fpfd;
        if f > 6e9 {
            self.set_pll_path(PllPath::Halved)?;
            n /= 2f32;
        } else {
            self.set_pll_path(PllPath::Direct)?;
        }
        self.set_divider_ratio(n)?;

        if n <= 512.0 {
            let caldiv = (fpfd / 250e3).floor() as u32;
            self.set_calibrator_division(caldiv)?;
        } else {
            panic!("Integer-only mode (N>=512) must be configured manually");
        }

        let mut st4: regs::ST4 = self.read_reg()?;
        match self.supply_voltage {
            crate::SupplyVoltage::LowVoltage => st4.vcalb_mode = true,
            crate::SupplyVoltage::HighVoltage => st4.vcalb_mode = f > 4500e6,
        };
        self.write_reg(&st4)?;

        Ok(())
    }

    /// Gets the PFD delay mode
    pub fn get_pfd_delay_mode(&mut self) -> Result<PfdDelayMode, Error<SPI, LE>> {
        let st3: regs::ST3 = self.read_reg()?;
        Ok(match st3.pfd_del_mode {
            0 => PfdDelayMode::NoDelay,
            1 => PfdDelayMode::VcoDivDelay,
            2 => PfdDelayMode::RefDivDelay,
            _ => unreachable!(),
        })
    }

    /// Sets the PFD delay mode
    /// It is recommended to set this to `VcoDivDelay`
    pub fn set_pfd_delay_mode(&mut self, mode: PfdDelayMode) -> Result<(), Error<SPI, LE>> {
        let mut st3: regs::ST3 = self.read_reg()?;
        st3.pfd_del_mode = mode as u32;
        self.write_reg(&st3)
    }

    /// Get the current PFD delay
    pub fn get_pfd_delay(&mut self) -> Result<PfdDelay, Error<SPI, LE>> {
        let st0: regs::ST0 = self.read_reg()?;
        Ok(match st0.pfd_del {
            0 => PfdDelay::Default,
            1 => PfdDelay::OneNine,
            3 => PfdDelay::TwoFive,
            4 => PfdDelay::ThreeZero,
            _ => unreachable!(),
        })
    }

    /// Sets the PFD delay
    /// It is recommended to set this to `Default`
    pub fn set_pfd_delay(&mut self, delay: PfdDelay) -> Result<(), Error<SPI, LE>> {
        let mut st0: regs::ST0 = self.read_reg()?;
        st0.pfd_del = delay as u32;
        self.write_reg(&st0)
    }

    /// Sets the charge pump scaling factor to 0..31*Imin
    pub fn set_charge_pump(&mut self, scale: u32) -> Result<(), Error<SPI, LE>> {
        assert!((scale <= 31), "Charge pump scale must be less than 32");
        let mut st0: regs::ST0 = self.read_reg()?;
        st0.cp_sel = scale;
        self.write_reg(&st0)
    }

    /// Gets the charge pump scaling factor
    pub fn get_charge_pump(&mut self) -> Result<u32, Error<SPI, LE>> {
        let st0: regs::ST0 = self.read_reg()?;
        Ok(st0.cp_sel)
    }

    // VCO Settings

    /// Sets the VCO calibrator division factor
    /// Must be between 0 and 511
    pub fn set_calibrator_division(&mut self, div: u32) -> Result<(), Error<SPI, LE>> {
        assert!(div <= 511, "VCO Calibrator division must be less than 512");
        let mut st6: regs::ST6 = self.read_reg()?;
        st6.cal_div = div;
        self.write_reg(&st6)
    }

    /// Gets the current VCO calibration division
    pub fn get_calibrator_division(&mut self) -> Result<u32, Error<SPI, LE>> {
        let st6: regs::ST6 = self.read_reg()?;
        Ok(st6.cal_div)
    }

    /// Gets the current VCO calibration frequency
    pub fn get_calibrator_frequency(&mut self) -> Result<f32, Error<SPI, LE>> {
        let st6: regs::ST6 = self.read_reg()?;
        let fpfd = self.get_pfd_frequency()?;
        Ok(fpfd / st6.cal_div as f32)
    }

    /// Set VCO amplitude
    /// Valid amplitude settings range from 0-2 for `LowVoltage` supply and 0-7 for `HighVoltage`
    /// It is recommended for phase noise's sake to set this to the maximum allowed by the supply
    /// Of course, a lower setting here reduces the power consumption
    pub fn set_vco_amplitude(&mut self, amplitude: u32) -> Result<(), Error<SPI, LE>> {
        match self.supply_voltage {
            crate::SupplyVoltage::LowVoltage => assert!(
                amplitude <= 2,
                "Low voltage supplies must have a maximum amplitude of 2"
            ),
            crate::SupplyVoltage::HighVoltage => {
                assert!(amplitude <= 7, "Amplitude has a maximum value of 7")
            }
        };
        let mut st4: regs::ST4 = self.read_reg()?;
        st4.vco_amp = amplitude;
        self.write_reg(&st4)
    }

    // Status stuff

    /// Gets the lock state of the PLL
    pub fn is_locked(&mut self) -> Result<bool, Error<SPI, LE>> {
        let st10: regs::ST10 = self.read_reg()?;
        Ok(st10.lock_det)
    }

    /// Returns true if all the cores startup properly
    pub fn is_startup(&mut self) -> Result<bool, Error<SPI, LE>> {
        let st10: regs::ST10 = self.read_reg()?;
        Ok(st10.reg_dig_startup
            && st10.reg_ref_startup
            && st10.reg_rf_startup
            && st10.reg_vco_4v5_startup)
    }

    /// Returns true if any of the cores threw an overcurrent flag
    pub fn is_ocp(&mut self) -> Result<bool, Error<SPI, LE>> {
        let st10: regs::ST10 = self.read_reg()?;
        Ok(st10.reg_dig_ocp || st10.reg_ref_ocp || st10.reg_rf_ocp || st10.reg_vco_4v5_ocp)
    }

    /* // TODO Fix this
    /// Dumps the contents of all the registers to stdout
    pub fn dump_regs(&mut self) -> Result<(), E> {
        let st0: regs::ST0 = self.read_reg()?;
        let st1: regs::ST1 = self.read_reg()?;
        let st2: regs::ST2 = self.read_reg()?;
        let st3: regs::ST3 = self.read_reg()?;
        let st4: regs::ST4 = self.read_reg()?;
        let st5: regs::ST5 = self.read_reg()?;
        let st6: regs::ST6 = self.read_reg()?;
        let st7: regs::ST7 = self.read_reg()?;
        let st8: regs::ST8 = self.read_reg()?;
        let st10: regs::ST10 = self.read_reg()?;

        println!("{:#?}", st0);
        println!("{:#?}", st1);
        println!("{:#?}", st2);
        println!("{:#?}", st3);
        println!("{:#?}", st4);
        println!("{:#?}", st5);
        println!("{:#?}", st6);
        println!("{:#?}", st7);
        println!("{:#?}", st8);
        println!("{:#?}", st10);
        Ok(())
    }
    */
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockStuw81300LE, MockStuw81300SPI};
    use embedded_hal_mock as mock;
    use mock::pin::{Mock as PinMock, State as PinState, Transaction as PinTransaction};
    use mock::spi::{Mock as SpiMock, Transaction as SpiTransaction};

    fn spi_tester(mosi: Vec<u8>, miso: Vec<u8>) -> STuW81300<SpiMock, PinMock> {
        let spi_expectations = [SpiTransaction::transfer(mosi, miso)];

        let pin_expectations = [
            PinTransaction::set(PinState::Low),
            PinTransaction::set(PinState::High),
        ];

        let spi = SpiMock::new(&spi_expectations);
        let le = PinMock::new(&pin_expectations);

        STuW81300 {
            spi,
            le,
            supply_voltage: crate::SupplyVoltage::HighVoltage,
            ref_freq: 100e6,
            ref_type: crate::ReferenceType::SingleEnded,
        }
    }

    fn mock_tester() -> STuW81300<MockStuw81300SPI, MockStuw81300LE> {
        STuW81300 {
            spi: MockStuw81300SPI::default(),
            le: MockStuw81300LE::default(),
            supply_voltage: crate::SupplyVoltage::HighVoltage,
            ref_freq: 100e6,
            ref_type: crate::ReferenceType::SingleEnded,
        }
    }

    #[test]
    fn device_id() {
        let mut vco = spi_tester(vec![0xd8, 0, 0, 0], vec![0, 0, 0x80, 0x52]);
        assert_eq!(vco.device_id().unwrap(), 0x8052);
    }

    #[test]
    fn mock_device_id() {
        let mut vco = mock_tester();
        assert_eq!(vco.device_id().unwrap(), 0x8052);
    }

    #[test]
    fn complete_mock() {
        let mut vco = mock_tester();
        vco.init().unwrap();

        vco.set_reference_clock_path(ReferenceClockPath::Direct)
            .unwrap();
        vco.set_reference_clock_divider(2).unwrap();
        assert_eq!(vco.get_pfd_frequency().unwrap(), 50e6);

        vco.set_dsm_order(DsmOrder::ThirdOrder).unwrap();
        vco.set_dithering(true).unwrap();
        vco.set_pfd_delay(PfdDelay::Default).unwrap();
        vco.set_pfd_delay_mode(PfdDelayMode::VcoDivDelay).unwrap();
        vco.set_vco_amplitude(7).unwrap();

        vco.set_output_frequency(7625e6).unwrap();
        assert_eq!(vco.get_output_frequency().unwrap(), 7625e6);

        vco.set_output_frequency(3151e6).unwrap();
        assert_eq!(vco.get_output_frequency().unwrap(), 3151e6);

        // 43.3 Hz of error in this case
        vco.set_output_frequency(3150123456.7).unwrap();
        assert_eq!(vco.get_output_frequency().unwrap(), 3150123500.0);

        vco.set_output_frequency(8e9).unwrap();
        assert_eq!(vco.get_output_frequency().unwrap(), 8e9);
        assert_eq!(vco.get_calibrator_frequency().unwrap(), 250e3);
    }
}
