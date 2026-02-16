#![no_std]
#![feature(ptr_as_uninit)]
#![feature(min_specialization)]
#![allow(clippy::missing_safety_doc)]

mod address;
mod capability;
pub mod common;
mod init;
mod irq;
pub mod list;
mod memlayout;
mod object;
mod riscv;
mod sbi;
mod scheduler;
mod syscall;
mod timer;
mod trap;
pub mod uart;

pub use capability::CapabilityType;
pub use common::{ErrKind, KernelError, KernelResult};
pub use init::init_kernel;
pub use object::Registers;
pub use syscall::InvLabel;
pub use syscall::SysCallNo;
pub use trap::return_to_user;
