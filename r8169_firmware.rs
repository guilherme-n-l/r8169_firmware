// SPDX-License-Identifier: GPL-2.0-only

/* SPDX-License-Identifier: GPL-2.0-only */
/* firmware.rs: RealTek 8169/8168/8101 ethernet driver.
 *
 * Copyright (c) 2025 Guilherme Lima <mail.guilhermenl@gmail.com>
 * Copyright (c) a lot of people too. Please respect their work.
 *
 * See MAINTAINERS file for support contact information.
 */

#![allow(missing_docs)]

mod r8169_macros;
use crate::r8169_macros::*;
use kernel::{bindings, device, ffi, firmware, prelude::*, str::CStr, types::ARef};

static FW_OPCODE_SIZE: usize = core::mem::size_of::<u32 /* typeof(RtlFwPhyAction::code[0]) */>();
static RTL_VER_SIZE: usize = 32;

type RtlFwWrite = *mut fn(&ffi::c_void, usize, i32);
type RtlFwRead = *mut fn(&ffi::c_void, usize) -> i32;

pub struct RtlFw {
    phy_write: RtlFwWrite,
    phy_read: RtlFwRead,
    mac_mcu_write: RtlFwWrite,
    mac_mcu_read: RtlFwRead,
    fw: Option<firmware::Firmware>,
    fw_name: *const ffi::c_char,
    dev: ARef<device::Device>,
    version: [u8; RTL_VER_SIZE],
    phy_action: RtlFwPhyAction,
}

define_rtl_fw_op_code! {RtlFwOpCode;
    PhyRead = 0x0,
    PhyDataOr = 0x1,
    PhyDataAnd = 0x2,
    PhyBjmpn = 0x3,
    PhyMdioChg = 0x4,
    PhyClearReadCount = 0x7,
    PhyWrite = 0x8,
    PhyReadcountEqSkip = 0x9,
    PhyCompEqSkipn = 0xa,
    PhyCompNeqSkipn = 0xb,
    PhyWritePrevious = 0xc,
    PhySkipn = 0xd,
    PhyDelayMs = 0xe
}

#[repr(C, packed)]
struct FwInfo {
    magic: u32,
    version: [u8; RTL_VER_SIZE],
    fw_start: u32, // __le32
    fw_len: u32,   // __le32
    chksum: u8,
}

impl FwInfo {
    fn from_bytes(bytes: &[u8], size: usize) -> Result<&Self> {
        if size < core::mem::size_of::<FwInfo>() || !(bytes.as_ptr() as *const FwInfo).is_aligned()
        {
            return Err(EINVAL);
        }
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }
}

#[repr(C)]
struct RtlFwPhyAction {
    code: *const u32,
    size: usize,
}

impl RtlFw {
    fn rtl_fw_format_ok(&mut self) -> bool {
        let fw = self.fw.as_ref().unwrap();
        let fw_info_data = fw.data();
        let fw_info_size = fw.size();
        let pa = &mut self.phy_action;

        if fw_info_size < FW_OPCODE_SIZE {
            return false;
        }

        let magic = u32::from_le_bytes(fw_info_data[0..4].try_into().unwrap());
        if magic == 0 {
            let checksum: u8 = fw_info_data.iter().sum();
            if checksum != 0 {
                return false;
            }
            let fw_info = match FwInfo::from_bytes(fw_info_data, fw_info_size) {
                Ok(info) => info,
                Err(_) => return false,
            };

            let start = u32::from_le(fw_info.fw_start) as usize;
            if start > fw_info_size {
                return false;
            }

            let size = u32::from_le(fw_info.fw_len) as usize;
            if size > (fw_info_size - start) / FW_OPCODE_SIZE {
                return false;
            }

            self.version.copy_from_slice(fw_info.version.as_ref());
            pa.code = unsafe { (fw_info_data.as_ptr() as *const u32).add(start) };
            pa.size = size;
        } else {
            if fw_info_size % FW_OPCODE_SIZE != 0 {
                return false;
            }
            self.version
                .copy_from_slice(unsafe { CStr::from_char_ptr(self.fw_name).as_bytes_with_nul() });
            pa.code = fw_info_data.as_ptr() as *const u32;
            pa.size = fw_info_size / FW_OPCODE_SIZE;
        }
        true
    }

    fn rtl_fw_data_ok(&self) -> bool {
        let pa = &self.phy_action;

        for index in 0..pa.size {
            let action = unsafe { u32::from_le(*pa.code.add(index)) };
            let val = (action & 0x0000ffff) as u16;
            let regno = ((action & 0x0fff0000) >> 16) as usize;
            let opcode = match RtlFwOpCode::from_u32(action >> 28) {
                Ok(op) => op,
                Err(_) => {
                    dev_err!(self.dev, "Invalid action 0x{:08x}\n", action);
                    return false;
                }
            };

            let out_of_range_when = |cond| {
                if cond {
                    dev_err!(self.dev, "Out of range of firmware\n");
                    return false;
                }
                true
            };

            if !match opcode {
                RtlFwOpCode::PhyRead
                | RtlFwOpCode::PhyDataOr
                | RtlFwOpCode::PhyDataAnd
                | RtlFwOpCode::PhyClearReadCount
                | RtlFwOpCode::PhyWrite
                | RtlFwOpCode::PhyWritePrevious
                | RtlFwOpCode::PhyDelayMs => true,
                RtlFwOpCode::PhyMdioChg => out_of_range_when(val > 1),
                RtlFwOpCode::PhyBjmpn => out_of_range_when(regno > index),
                RtlFwOpCode::PhyReadcountEqSkip => out_of_range_when(index + 2 >= pa.size),
                RtlFwOpCode::PhyCompEqSkipn
                | RtlFwOpCode::PhyCompNeqSkipn
                | RtlFwOpCode::PhySkipn => out_of_range_when(index + 1 + regno >= pa.size),
            } {
                return false;
            }
        }
        true
    }

    fn rtl_fw_write_firmware(&self, tp: &ffi::c_void) {
        let pa = &self.phy_action;
        let mut fw_write = self.phy_write;
        let mut fw_read = self.phy_read;
        let (mut predata, mut count): (i32, i32) = (0, 0);
        let mut index: usize = 0;
        while index < pa.size {
            let action = unsafe { u32::from_le(*pa.code.add(index)) };
            let data = action & 0x0000ffff;
            let regno = ((action & 0x0fff0000) >> 16) as usize;
            let opcode = match RtlFwOpCode::from_u32(action >> 28) {
                Ok(op) => op,
                Err(_) => {
                    dev_err!(self.dev, "Invalid action 0x{:08x}", action);
                    return;
                }
            };

            match opcode {
                RtlFwOpCode::PhyRead => {
                    predata = unsafe { (*fw_read)(tp, regno) };
                    count += 1;
                }
                RtlFwOpCode::PhyDataOr => {
                    predata |= data as i32;
                }
                RtlFwOpCode::PhyDataAnd => {
                    predata &= data as i32;
                }
                RtlFwOpCode::PhyBjmpn => {
                    index -= regno + 1;
                }
                RtlFwOpCode::PhyMdioChg => {
                    if data != 0 {
                        fw_write = self.mac_mcu_write;
                        fw_read = self.mac_mcu_read;
                    } else {
                        fw_write = self.phy_write;
                        fw_read = self.phy_read;
                    }
                }
                RtlFwOpCode::PhyClearReadCount => {
                    count = 0;
                }
                RtlFwOpCode::PhyWrite => unsafe {
                    (*fw_write)(tp, regno, data as i32);
                },
                RtlFwOpCode::PhyReadcountEqSkip => {
                    if count == data as i32 {
                        index += 1
                    }
                }
                RtlFwOpCode::PhyCompEqSkipn => {
                    if predata == data as i32 {
                        index += regno;
                    }
                }
                RtlFwOpCode::PhyCompNeqSkipn => {
                    if predata != data as i32 {
                        index += regno;
                    }
                }
                RtlFwOpCode::PhyWritePrevious => unsafe {
                    (*fw_write)(tp, regno, predata);
                },
                RtlFwOpCode::PhySkipn => {
                    index += regno;
                }
                RtlFwOpCode::PhyDelayMs => unsafe {
                    bindings::msleep(data);
                },
            }
        }
    }

    fn rtl_fw_request_firmware(&mut self) -> i32 {
        let fw_name = self.fw_name.clone();
        let dev = self.dev.clone();
        let out = |rc: i32| {
            dev_warn!(
                dev,
                "Unable to load firmware {} ({})",
                unsafe { CStr::from_char_ptr(fw_name).to_str().unwrap() },
                rc
            );
            rc
        };
        match firmware::Firmware::request_nowarn(
            unsafe { CStr::from_char_ptr(self.fw_name) },
            &self.dev,
        )
        .map_err(|err| out(err.to_errno()))
        {
            Ok(fw) => self.fw = Some(fw),
            Err(rc) => return rc,
        }

        if !self.rtl_fw_format_ok() || !self.rtl_fw_data_ok() {
            self.rtl_fw_release_firmware();
            return out(EINVAL.to_errno());
        }

        0
    }

    fn rtl_fw_release_firmware(&mut self) {
        self.fw = None;
    }
}

define_rtl_c_fn! {
    rtl_fw_format_ok() -> bool;
    rtl_fw_data_ok() -> bool;
    rtl_fw_write_firmware(tp: &ffi::c_void);
    rtl_fw_request_firmware() -> i32;
    rtl_fw_release_firmware()
}
