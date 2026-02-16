#![no_std]

use core::fmt;

use syscall::put_char;
pub mod shared;

pub use crate::shared::err_kind::ErrKind;
pub use crate::shared::inv_labels::InvLabel;
pub use crate::shared::registers::Registers;
pub use crate::shared::syscall_no::SysCallNo;
pub use crate::shared::types::BootInfo;
pub use crate::shared::types::IPCBuffer;
pub use crate::shared::types::UntypedInfo;
pub mod caps;
pub mod syscall;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

pub struct SyscallWriter;

impl fmt::Write for SyscallWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for ch in s.as_bytes() {
            let ch = *ch;
            put_char(ch).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = SyscallWriter;
    use fmt::Write;
    let _ = writer.write_fmt(args);
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
