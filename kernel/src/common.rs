use core::{cell::UnsafeCell, error::Error, fmt};
pub use shared::align_down;
pub use shared::align_up;
pub use shared::err_kind::ErrKind;
pub use shared::is_aligned;
pub use shared::types::BootInfo;
pub use shared::types::IPCBuffer;
pub use shared::types::UntypedInfo;

pub type KernelResult<T> = Result<T, KernelError>;

#[repr(transparent)]
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

impl<T> SyncUnsafeCell<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub const fn get(&self) -> *mut T {
        self.0.get()
    }
}

// SAFETY: Kernel runtime is single-core for mutable access assumptions here.
unsafe impl<T> Sync for SyncUnsafeCell<T> {}

//TODO: thiserror and anyhow
#[derive(Debug)]
pub struct KernelError {
    pub e_kind: ErrKind,
    pub e_val: u16,
    #[cfg(debug_assertions)]
    pub e_place: EPlace,
}

#[macro_export]
macro_rules! kerr {
    ($ekind:expr) => {
        $crate::common::KernelError {
            e_kind: $ekind,
            e_val: 0,
            #[cfg(debug_assertions)]
            e_place: $crate::common::EPlace {
                e_line: line!(),
                e_column: column!(),
                e_file: file!(),
            },
        }
    };

    ($ekind:expr, $eval:expr) => {
        $crate::common::KernelError {
            e_kind: $ekind,
            e_val: $eval,
            #[cfg(debug_assertions)]
            e_place: $crate::common::EPlace {
                e_line: line!(),
                e_column: column!(),
                e_file: file!(),
            },
        }
    };
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for KernelError {}

#[cfg(debug_assertions)]
#[derive(Debug)]
pub struct EPlace {
    pub e_line: u32,
    pub e_column: u32,
    pub e_file: &'static str,
}
