use crate::ErrKind;
use crate::IPCBuffer;
use crate::InvLabel;
use crate::Registers;
use crate::SysCallNo;
use core::arch::asm;
pub use shared::cap_type::CapabilityType;

pub type SysCallRes = Result<usize, SysCallFailed>;
pub type SysCallFailed = (ErrKind, u16);

#[allow(clippy::too_many_arguments)]
unsafe fn syscall(
    cap_ptr: usize,
    cap_depth: u32,
    inv_label: InvLabel,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
    sysno: SysCallNo,
) -> SysCallRes {
    let mut is_error: usize;
    let mut val: usize;

    asm!(
        "ecall",
        inout("a0") cap_ptr => is_error,
        inout("a1") cap_depth as usize => val,
        in("a2") inv_label as usize,
        in("a3") arg3,
        in("a4") arg4,
        in("a5") arg5,
        in("a6") arg6,
        in("a7") sysno as usize,
    );

    if is_error == 0 {
        Ok(val)
    } else {
        let e_kind = ErrKind::try_from(is_error).unwrap_or(ErrKind::UnknownSysCall);
        Err((e_kind, val as u16))
    }
}

pub fn put_char(char: u8) -> SysCallRes {
    unsafe {
        syscall(
            char as usize,
            0,
            InvLabel::PutChar,
            0,
            0,
            0,
            0,
            SysCallNo::Print,
        )
    }
}

pub fn traverse() -> SysCallRes {
    unsafe { syscall(0, 0, InvLabel::CNodeTraverse, 0, 0, 0, 0, SysCallNo::Print) }
}

#[allow(clippy::too_many_arguments)]
pub fn untyped_retype(
    cap_ptr: usize,
    cap_depth: u32,
    dest_ptr: usize,
    dest_depth: u32,
    index: u32,
    user_size: u32,
    num: u32,
    cap_type: CapabilityType,
) -> SysCallRes {
    let index_and_depth = ((index as usize) << 32) | dest_depth as usize;
    let user_size_and_num = ((user_size as usize) << 32) | num as usize;
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::UntypedRetype,
            dest_ptr,
            index_and_depth,
            user_size_and_num,
            cap_type as usize,
            SysCallNo::Call,
        )
    }
}

pub fn write_reg<F>(
    cap_ptr: usize,
    cap_depth: u32,
    register: F,
    buffer: &mut IPCBuffer,
) -> SysCallRes
where
    F: FnOnce() -> Registers,
{
    buffer
        .write_as(register)
        .map_err(|_| (ErrKind::InvalidOperation, 0))?;

    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::TcbWriteReg,
            0,
            0,
            0,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn set_ipc_buffer(
    cap_ptr: usize,
    cap_depth: u32,
    page_cap_ptr: usize,
    page_depth: u32,
) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::TcbSetIpcBuffer,
            page_cap_ptr,
            page_depth as usize,
            0,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn configure_tcb(
    cap_ptr: usize,
    cap_depth: u32,
    cnode_ptr: usize,
    cnode_depth: u32,
    vspace_ptr: usize,
    vspace_depth: u32,
) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::TcbConfigure,
            cnode_ptr,
            cnode_depth as usize,
            vspace_ptr,
            vspace_depth as usize,
            SysCallNo::Call,
        )
    }
}

pub fn resume_tcb(cap_ptr: usize, cap_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::TcbResume,
            0,
            0,
            0,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn send_signal(cap_ptr: usize, cap_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::NotifySend,
            0,
            0,
            0,
            0,
            SysCallNo::Send,
        )
    }
}

pub fn recv_signal(cap_ptr: usize, cap_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::NotifyWait,
            0,
            0,
            0,
            0,
            SysCallNo::Recv,
        )
    }
}

pub fn cnode_copy(
    cap_ptr: usize,
    cap_depth: u32,
    dest_index: usize,
    dest_depth: u32,
    src_index: usize,
    src_depth: u32,
) -> SysCallRes {
    let depth = ((src_depth as usize) << 32) | dest_depth as usize;
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::CNodeCopy,
            src_index,
            depth,
            dest_index,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn cnode_mint(
    cap_ptr: usize,
    cap_depth: u32,
    dest_index: usize,
    dest_depth: u32,
    src_index: usize,
    src_depth: u32,
    cap_val: usize,
) -> SysCallRes {
    let depth = ((src_depth as usize) << 32) | dest_depth as usize;
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::CNodeMint,
            src_index,
            depth,
            dest_index,
            cap_val,
            SysCallNo::Call,
        )
    }
}

pub fn cnode_move(
    cap_ptr: usize,
    cap_depth: u32,
    dest_depth: u32,
    dest_index: usize,
    src_index: usize,
    src_depth: u32,
) -> SysCallRes {
    let depth = ((src_depth as usize) << 32) | dest_depth as usize;
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::CNodeMove,
            src_index,
            depth,
            dest_index,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn map_page(
    cap_ptr: usize,
    cap_depth: u32,
    dest_ptr: usize,
    dest_depth: u32,
    vaddr: usize,
    flags: usize,
) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::PageMap,
            dest_ptr,
            dest_depth as usize,
            vaddr,
            flags,
            SysCallNo::Call,
        )
    }
}

pub fn unmap_page(cap_ptr: usize, cap_depth: u32, dest_ptr: usize, dest_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::PageUnMap,
            dest_ptr,
            dest_depth as usize,
            0,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn map_page_table(
    cap_ptr: usize,
    cap_depth: u32,
    dest_ptr: usize,
    dest_depth: u32,
    vaddr: usize,
) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::PageTableMap,
            dest_ptr,
            dest_depth as usize,
            vaddr,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn make_page_table_root(cap_ptr: usize, cap_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::PageTableMakeRoot,
            0,
            0,
            0,
            0,
            SysCallNo::Call,
        )
    }
}

pub fn send_ipc(cap_ptr: usize, cap_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::EpSend,
            0,
            0,
            0,
            0,
            SysCallNo::Send,
        )
    }
}

pub fn recv_ipc(cap_ptr: usize, cap_depth: u32) -> SysCallRes {
    unsafe {
        syscall(
            cap_ptr,
            cap_depth,
            InvLabel::EpRecv,
            0,
            0,
            0,
            0,
            SysCallNo::Recv,
        )
    }
}
