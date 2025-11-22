// SPDX-License-Identifier: GPL-2.0-only

/* SPDX-License-Identifier: GPL-2.0-only */
/* r8169_macros.rs: RealTek 8169/8168/8101 ethernet driver.
 *
 * Copyright (c) 2025 Guilherme Lima <mail.guilhermenl@gmail.com>
 * Copyright (c) a lot of people too. Please respect their work.
 *
 * See MAINTAINERS file for support contact information.
 */

macro_rules! define_rtl_fw_op_code {
    ($enum_name:ident; $($name:ident = $val:expr),*) => {
        #[derive(Clone)]
        #[repr(u32)]
        enum $enum_name {
            $(
                $name = $val,
            )*
        }

        impl $enum_name {
            fn from_u32(n: u32) -> Result<Self> {
                match n {
                    $(
                        $val => Ok(Self::$name),
                    )*
                    _ => Err(EINVAL),
                }
            }
        }
    }
}
pub(crate) use define_rtl_fw_op_code;

macro_rules! define_rtl_c_fn {
    ($($fn_name:ident($($arg:ident : $arg_type:ty),*)$( -> $ret:ty)?);*) => {
        $(
        #[no_mangle]
        pub fn $fn_name(fw: *mut RtlFw$(, $arg : $arg_type)*)$( -> $ret)? {
            unsafe { (*fw).$fn_name($($arg)*) }
        }
        )*
    };
}

pub(crate) use define_rtl_c_fn;
