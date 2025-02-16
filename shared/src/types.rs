use crate::const_assert;
use crate::err_kind::ErrKind;
use crate::registers::Registers;
use core::mem;

pub struct IPCBuffer {
    pub tag: usize,
    pub message: [usize; MESSAGE_LEN],
    pub user_data: usize,
}

impl IPCBuffer {
    pub fn write_as<F, T>(&mut self, write_fn: F) -> Result<(), ErrKind>
    where
        F: FnOnce() -> T,
        T: Sized,
    {
        (size_of_val(&self.message) >= mem::size_of::<T>())
            .then_some(())
            .ok_or(ErrKind::InvalidOperation)?;
        let ptr = &mut self.message[0] as *mut usize as *mut T;
        unsafe {
            *ptr = write_fn();
        }
        Ok(())
    }

    pub fn read_as<T: Sized>(&self) -> Result<&T, ErrKind> {
        (size_of_val(&self.message) >= mem::size_of::<T>())
            .then_some(())
            .ok_or(ErrKind::InvalidOperation)?;
        let ptr = &self.message[0] as *const usize as *const T;
        unsafe { Ok(&*ptr) }
    }
}
// bits, idx, is_device
#[derive(Default, Debug)]
pub struct UntypedInfo {
    pub bits: usize,
    pub idx: usize,
    pub is_device: bool,
    pub phys_addr: usize
}

#[derive(Default, Debug)]
pub struct BootInfo {
    pub ipc_buffer_addr: usize,
    pub root_cnode_idx: usize,
    pub root_vspace_idx: usize,
    pub untyped_num: usize,
    pub firtst_empty_idx: usize,
    pub msg: [u8; 32],
    pub untyped_infos: [UntypedInfo; 32],
}

impl BootInfo {
    #[allow(clippy::mut_from_ref)]
    pub fn ipc_buffer(&self) -> &mut IPCBuffer {
        let ptr = self.ipc_buffer_addr as *mut IPCBuffer;
        unsafe { &mut *ptr }
    }
}

pub const MESSAGE_LEN: usize = 128;
const_assert!(
    mem::size_of::<BootInfo>() <= 4096,
    mem::size_of::<IPCBuffer>() <= 4096,
    mem::size_of::<Registers>() <= mem::size_of::<[usize; MESSAGE_LEN]>()
);
