#[repr(u8)]
#[derive(Debug, PartialEq)]
pub(crate) enum RegisterAddr {
    // Master register. N divider, CP current
    ST0,
    // FRAC value, RF1 output control
    ST1,
    // MOD value, RF2 output control
    ST2,
    // R divider, CP leakage, CP down-split pulse, Ref. path selection, Device power down
    ST3,
    // Lock det. control, Ref. Buffer, CP supply mode, VCO settings, output power control
    ST4,
    // Low power mode control bit
    ST5,
    // VCO calibrator, manual vco control, DSM settings
    ST6,
    // Fast lock control, LD_SDO settings
    ST7,
    // LDO voltage regulator settings
    ST8,
    // Reserved - Test and initialization bit
    ST9,
    // VCO, lock det. status, LDO status
    ST10,
    // Device ID
    ST11,
}

impl RegisterAddr {
    pub(crate) fn read_only(&self) -> bool {
        matches!(self, RegisterAddr::ST10 | RegisterAddr::ST11)
    }
}

pub(crate) trait Register {
    fn addr(&self) -> RegisterAddr;
}

// Utilities

fn shift_flag_forward(flag: bool, bit: u8) -> u32 {
    (flag as u32) << bit
}

fn shift_num_forward(num: u32, start: u8) -> u32 {
    num << start
}

fn shift_flag_back(payload: u32, bit: u8) -> bool {
    (payload & 2_u32.pow(bit as u32)) >> bit == 1
}

fn shift_num_back(payload: u32, start: u8, size: u8) -> u32 {
    (payload >> start) & (2u32.pow(size as u32) - 1)
}

macro_rules! register {
    ($name:ident,
     numbers:
     {
         $($num:ident : ($size:literal, $start:literal),)*
     },
     flags:
     {
        $($flag:ident: $pos:literal,)*
     }) => {
        #[derive(Debug, PartialEq)]
        struct $name {
            $($num: u32,)*
            $($flag: bool,)*
        }
        impl Register for $name {
            fn addr(&self) -> RegisterAddr {
                RegisterAddr::$name
            }
        }
        impl From<&$name> for u32 {
            fn from(register: &$name) -> Self {
                #[cfg(not(release))]
                {
                    $(assert!(register.$num < 2u32.pow($size), concat!(stringify!($num), " must be ", stringify!($size), " bits!"));)*
                }
                let mut payload = 0u32;
                $(payload |= shift_num_forward(register.$num,$start);)*
                $(payload |= shift_flag_forward(register.$flag,$pos);)*
                payload
            }
        }
        impl From<u32> for $name {
            fn from(payload: u32) -> Self {
                $(let $num = shift_num_back(payload,$start,$size);)*
                $(let $flag = shift_flag_back(payload,$pos);)*
                Self {
                    $($num,)*
                    $($flag,)*
                }
            }
        }
    };
}

register!(
    ST0,
    numbers:
    {
        cp_sel: (5, 21),
        pfd_del: (2, 19),
        n: (17, 0),
    },
    flags:
    {
    }
);

register!(
    ST1,
    numbers:
    {
        frac: (21,0),
    },
    flags:
    {
        dbr: 26,
        rf1_out_pd: 24,
        man_calb_en: 23,
        pll_sel: 22,
        rf1_sel: 21,
    }
);

register!(
    ST2,
    numbers:
    {
        modu: (21,0),
    },
    flags:
    {
        dbr: 26,
        rf2_out_pd: 21,
    }
);

register!(
    ST3,
    numbers:
    {
        cp_leak: (5,19),
        pfd_del_mode: (2,15),
        ref_path_sel: (2,13),
        r: (13,0),
    },
    flags:
    {
        dbr: 26,
        pd: 25,
        cp_leak_x2: 24,
        cp_leak_dir: 18,
        dnsplit_en: 17,
    }
);

register!(
    ST4,
    numbers:
    {
        vco_amp: (3,15),
        ref_buff_mode: (2,8),
        ld_prec: (3,3),
        ld_count: (3,0),
    },
    flags:
    {
        calb_3v3_mode1: 24,
        rf_out_3v3: 23,
        ext_vco_en: 19,
        calb_3v3_mode0: 14,
        vcalb_mode: 12,
        kvco_comp_dis: 11,
        pfd_pol: 10,
        mute_lock_en: 7,
        ld_activelow: 6,
    }
);

register!(
    ST5,
    numbers:
    {
    },
    flags:
    {
        rf2_outbuf_lp: 4,
        demux_lp: 2,
        ref_buff_lp: 0,
    }
);

register!(
    ST6,
    numbers:
    {
        dsm_order: (2,22),
        prchg_del: (2,10),
        cal_div: (9,0),
    },
    flags:
    {
        dithering: 26,
        en_autocal: 20,
        cal_temp_comp: 12,
        cal_acc_en: 9,
    }
);

register!(
    ST7,
    numbers:
    {
        cp_sel_fl: (5,13),
        fstlck_cnt: (13,0),
    },
    flags:
    {
        ld_sdo_tristate: 25,
        ld_sdo_mode: 24,
        spi_data_out_disable: 23,
        cycle_slip_en: 19,
        fstlck_en: 18,
    }
);

register!(
    ST8,
    numbers:
    {
        reg_vco_4v5_vout: (2,0),
    },
    flags:
    {
        pd_rf2_disable: 26,
    }
);

register!(
    ST10,
    numbers:
    {
        vco_sel: (2,5),
        word: (5,0),
    },
    flags:
    {
        reg_dig_startup: 17,
        reg_ref_startup: 16,
        reg_rf_startup: 15,
        reg_vco_startup: 14,
        reg_vco_4v5_startup: 13,
        reg_dig_ocp: 12,
        reg_ref_ocp: 11,
        reg_rf_ocp: 10,
        reg_vco_ocp: 9,
        reg_vco_4v5_ocp: 8,
        lock_det: 7,
    }
);

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn round_trip_st0(n in 0u32..131071u32, cp_sel in 0u32..31u32, pfd_del in 0u32..3u32) {
            let st0 = ST0 {
                cp_sel,
                pfd_del,
                n,
            };
            let rt: ST0 = Into::<u32>::into(&st0).into();
            assert_eq!(rt, st0);
        }

        #[test]
        fn round_trip_st1(frac in 0u32..2097151u32,
                          dbr: bool,
                          rf1_out_pd: bool,
                          man_calb_en: bool,
                          pll_sel: bool,
                          rf1_sel: bool) {
            let st1 = ST1 {
                frac,
                dbr,
                rf1_out_pd,
                man_calb_en,
                pll_sel,
                rf1_sel,
            };
            let rt: ST1 = Into::<u32>::into(&st1).into();
            assert_eq!(rt, st1);
        }

        #[test]
        fn round_trip_st2(dbr: bool, rf2_out_pd: bool, modu in 2u32..2097151u32) {
            let st2 = ST2 {
                dbr,
                rf2_out_pd,
                modu,
            };
            let rt: ST2 = Into::<u32>::into(&st2).into();
            assert_eq!(rt, st2);
        }

        #[test]
        fn round_trip_st3(dbr: bool,
                          pd: bool,
                          cp_leak_x2: bool,
                          cp_leak in 0u32..31u32,
                          cp_leak_dir: bool,
                          dnsplit_en: bool,
                          pfd_del_mode in 0u32..3u32,
                          ref_path_sel in 0u32..3u32,
                          r in 1u32..8191u32) {
            let st3 = ST3 {dbr,pd, cp_leak, pfd_del_mode, ref_path_sel, r, cp_leak_x2, cp_leak_dir, dnsplit_en };
            let rt: ST3 = Into::<u32>::into(&st3).into();
            assert_eq!(rt, st3);
        }

        #[test]
        fn round_trip_st4(calb_3v3_mode1: bool,
                          rf_out_3v3: bool,
                          ext_vco_en: bool,
                          vco_amp in 0u32..7u32,
                          calb_3v3_mode0: bool,
                          vcalb_mode: bool,
                          kvco_comp_dis: bool,
                          pfd_pol: bool,
                          ref_buff_mode in 0u32..3u32,
                          mute_lock_en: bool,
                          ld_activelow: bool,
                          ld_prec in 0u32..7u32,
                          ld_count in 0u32..7u32) {
            let st4 = ST4 { vco_amp, ref_buff_mode, ld_prec, ld_count, calb_3v3_mode1, rf_out_3v3, ext_vco_en, calb_3v3_mode0, vcalb_mode, kvco_comp_dis, pfd_pol, mute_lock_en, ld_activelow };
            let rt: ST4 = Into::<u32>::into(&st4).into();
            assert_eq!(rt, st4);
        }

        #[test]
        fn round_trip_st5(rf2_outbuf_lp: bool,demux_lp: bool,ref_buff_lp: bool) {
            let st5 = ST5 { rf2_outbuf_lp, demux_lp, ref_buff_lp };
            let rt: ST5 = Into::<u32>::into(&st5).into();
            assert_eq!(rt,st5);
        }

        #[test]
        fn round_trip_st6(dithering: bool,
                          dsm_order in 0u32..3u32,
                          en_autocal: bool,
                          cal_temp_comp: bool,
                          prchg_del in 0u32..3u32,
                          cal_acc_en: bool,
                          cal_div in 1u32..511u32) {
            let st6 = ST6 { dsm_order, prchg_del, cal_div, dithering, en_autocal, cal_temp_comp, cal_acc_en };
            let rt: ST6 = Into::<u32>::into(&st6).into();
            assert_eq!(rt,st6);
        }

        #[test]
        fn round_trip_st7(ld_sdo_tristate: bool,
                          ld_sdo_mode: bool,
                          spi_data_out_disable: bool,
                          cycle_slip_en: bool,
                          fstlck_en: bool,
                          cp_sel_fl in 0u32..31u32,
                          fstlck_cnt in 2u32..8191u32) {
            let st7 = ST7 { cp_sel_fl, fstlck_cnt, ld_sdo_tristate, ld_sdo_mode, spi_data_out_disable, cycle_slip_en, fstlck_en };
            let rt: ST7 = Into::<u32>::into(&st7).into();
            assert_eq!(rt,st7);
        }

        #[test]
        fn round_trip_st8(pd_rf2_disable: bool, reg_vco_4v5_vout in 2u32..3u32) {
            let st8 = ST8 { reg_vco_4v5_vout, pd_rf2_disable };
            let rt: ST8 = Into::<u32>::into(&st8).into();
            assert_eq!(rt,st8);
        }

        #[test]
        fn round_trip_st10(vco_sel in 0u32..3u32,
                           word in 0u32..31u32,
                           reg_dig_startup: bool,
                           reg_ref_startup: bool,
                           reg_rf_startup: bool,
                           reg_vco_startup: bool,
                           reg_vco_4v5_startup: bool,
                           reg_dig_ocp: bool,
                           reg_ref_ocp: bool,
                           reg_rf_ocp: bool,
                           reg_vco_ocp: bool,
                           reg_vco_4v5_ocp: bool,
                           lock_det: bool) {
            let st10 = ST10 { vco_sel, word, reg_dig_startup, reg_ref_startup, reg_rf_startup, reg_vco_startup, reg_vco_4v5_startup, reg_dig_ocp, reg_ref_ocp, reg_rf_ocp, reg_vco_ocp, reg_vco_4v5_ocp, lock_det };
            let rt: ST10 = Into::<u32>::into(&st10).into();
            assert_eq!(rt,st10);
        }
    }
}
