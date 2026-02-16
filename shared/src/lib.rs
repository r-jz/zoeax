#![no_std]

pub mod aligned_to;
pub mod cap_type;
pub mod elf;
pub mod err_kind;
pub mod inv_labels;
pub mod registers;
pub mod syscall_no;
pub mod types;

pub const PAGE_SIZE: usize = 4096;

#[macro_export]
macro_rules! const_assert_single {
    ($cond:expr, $msg:expr $(,)?) => {
        const _: () = {
            if !$cond {
                panic!($msg);
            }
        };
    };
    ($cond:expr $(,)?) => {
        const _: () = {
            if !$cond {
                panic!(concat!(
                    "Compile-time assertion failed: ",
                    stringify!($cond)
                ));
            }
        };
    };
}

#[macro_export]
macro_rules! const_assert {
    ($($cond:expr),+ $(,)?) => {
        $( $crate::const_assert_single!($cond); )+
    };
    ( $( $cond:expr => $msg:expr ),+ $(,)? ) => {
        $( $crate::const_assert_single!($cond, $msg); )+
    };
}

pub fn is_aligned(value: usize, align: usize) -> bool {
    value.is_multiple_of(align)
}

/// align should be power of 2.
pub fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// align should be power of 2.
pub fn align_down(value: usize, align: usize) -> usize {
    (value) & !(align - 1)
}
