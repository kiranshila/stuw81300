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
        vco_calb_disable: 26,
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
        ref_buf_mode: (2,8),
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
        cal_acc_en: 1,
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

    #[test]
    fn round_trip_st0() {
        let st0 = ST0 {
            vco_calb_disable: true,
            cp_sel: 31,
            pfd_del: 2,
            n: 0x0000DEAD,
        };

        let rt: ST0 = Into::<u32>::into(&st0).into();
        assert_eq!(rt, st0);
    }
}
