use crate::error::Error;
use crate::registers::{Register, RegisterAddr};
use crate::STuW81300;
use embedded_hal as hal;
use hal::blocking::spi::Transfer;
use hal::digital::v2::OutputPin;

#[repr(u8)]
#[derive(Debug, PartialEq)]
enum AccessMode {
    Write = 0,
    Read = 1,
}

impl<SPI, LE> STuW81300<SPI, LE>
where
    SPI: Transfer<u8>,
    LE: OutputPin,
{
    fn operate(
        &mut self,
        addr: RegisterAddr,
        data: u32,
        mode: AccessMode,
    ) -> Result<u32, Error<SPI, LE>> {
        // Pack data
        let mut buf = pack(addr, data, mode);
        // Perform transaction. Do we care about timing?
        self.le.set_low().map_err(|e| Error::LatchEnable(e))?;
        self.spi
            .transfer(&mut buf)
            .map_err(|e| Error::Transfer(e))?;
        self.le.set_high().map_err(|e| Error::LatchEnable(e))?;
        // Extract data
        Ok(u32::from_be_bytes(buf))
    }

    pub(crate) fn read(&mut self, addr: RegisterAddr) -> Result<u32, Error<SPI, LE>> {
        self.operate(addr, 0, AccessMode::Read)
    }

    pub(crate) fn write(&mut self, addr: RegisterAddr, data: u32) -> Result<(), Error<SPI, LE>> {
        self.operate(addr, data, AccessMode::Write)?;
        Ok(())
    }

    pub(crate) fn read_reg<R>(&mut self) -> Result<R, Error<SPI, LE>>
    where
        R: Register + From<u32>,
    {
        self.read(R::addr()).map(Into::into)
    }

    pub(crate) fn write_reg<'a, R>(&mut self, register: &'a R) -> Result<(), Error<SPI, LE>>
    where
        R: Register,
        &'a R: Into<u32>,
    {
        self.write(R::addr(), register.into())
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

// Tests

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

        STuW81300 {
            spi,
            le,
            supply_voltage: crate::SupplyVoltage::HighVoltage,
            ref_freq: 100e6,
            ref_type: crate::ReferenceType::SingleEnded,
        }
    }

    #[test]
    fn write_st8() {
        let mut vco = spi_tester(vec![0x40, 0, 0, 3], vec![0, 0, 0, 0]);

        let st8 = crate::registers::ST8 {
            reg_vco_4v5_vout: 3,
            pd_rf2_disable: false,
        };

        vco.write_reg(&st8).unwrap();
    }

    #[test]
    fn write_st7() {
        let mut vco = spi_tester(vec![0x39, 0, 0, 0], vec![0, 0, 0, 0]);

        let st7 = crate::registers::ST7 {
            cp_sel_fl: 0,
            fstlck_cnt: 0,
            ld_sdo_tristate: false,
            ld_sdo_mode: true,
            spi_data_out_disable: false,
            cycle_slip_en: false,
            fstlck_en: false,
        };

        vco.write_reg(&st7).unwrap();
    }

    #[test]
    fn write_st6() {
        let mut vco = spi_tester(vec![0x30, 0, 0x10, 0], vec![0, 0, 0, 0]);

        let st6 = crate::registers::ST6 {
            dsm_order: 0,
            prchg_del: 0,
            cal_div: 0,
            dithering: false,
            en_autocal: false,
            cal_temp_comp: true,
            cal_acc_en: false,
        };

        vco.write_reg(&st6).unwrap();
    }

    #[test]
    fn write_st5() {
        let mut vco = spi_tester(vec![0x28, 0, 0, 0], vec![0, 0, 0, 0]);

        let st5 = crate::registers::ST5 {
            rf2_outbuf_lp: false,
            demux_lp: false,
            ref_buff_lp: false,
        };

        vco.write_reg(&st5).unwrap();
    }

    #[test]
    fn write_st4() {
        let mut vco = spi_tester(vec![0x20, 0x03, 0x93, 0x15], vec![0, 0, 0, 0]);

        let st4 = crate::registers::ST4 {
            vco_amp: 7,
            ref_buff_mode: 3,
            ld_prec: 2,
            ld_count: 5,
            calb_3v3_mode1: false,
            rf_out_3v3: false,
            ext_vco_en: false,
            calb_3v3_mode0: false,
            vcalb_mode: true,
            kvco_comp_dis: false,
            pfd_pol: false,
            mute_lock_en: false,
            ld_activelow: false,
        };

        vco.write_reg(&st4).unwrap();
    }

    #[test]
    fn write_st3() {
        let mut vco = spi_tester(vec![0x18, 0, 0x80, 0x02], vec![0, 0, 0, 0]);

        let st3 = crate::registers::ST3 {
            pfd_del_mode: 1,
            ref_path_sel: 0,
            r: 2,
            cp_leak: 0,
            dbr: false,
            pd: false,
            cp_leak_x2: false,
            cp_leak_dir: false,
            dnsplit_en: false,
        };

        vco.write_reg(&st3).unwrap();
    }

    #[test]
    fn write_st2() {
        let mut vco = spi_tester(vec![0x10, 0, 0, 0x0A], vec![0, 0, 0, 0]);

        let st2 = crate::registers::ST2 {
            modu: 10,
            dbr: false,
            rf2_out_pd: false,
        };

        vco.write_reg(&st2).unwrap();
    }

    #[test]
    fn write_st1() {
        let mut vco = spi_tester(vec![0x09, 0x40, 0, 0x01], vec![0, 0, 0, 0]);

        let st1 = crate::registers::ST1 {
            frac: 1,
            dbr: false,
            rf1_out_pd: true,
            man_calb_en: false,
            pll_sel: true,
            rf1_sel: false,
        };

        vco.write_reg(&st1).unwrap();
    }

    #[test]
    fn write_st0() {
        let mut vco = spi_tester(vec![0x03, 0xE0, 0x00, 0x4C], vec![0, 0, 0, 0]);

        let st0 = crate::registers::ST0 {
            cp_sel: 31,
            pfd_del: 0,
            n: 76,
        };

        vco.write_reg(&st0).unwrap();
    }
}
