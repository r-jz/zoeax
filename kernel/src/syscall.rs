use crate::{
    address::PAGE_SIZE,
    capability::{cap_try_from_u8, CapabilityType},
    common::{is_aligned, ErrKind, KernelResult},
    kerr,
    object::{
        get_user_flags,
        page_table::{Page, PAGE_U},
        CNode, CNodeEntry, Endpoint, Notification, PageTable, Registers, ThreadControlBlock,
        Untyped,
    },
    println,
    scheduler::{get_current_tcb_mut, require_schedule},
    uart::putchar,
};
pub use shared::inv_labels::InvLabel;
pub use shared::syscall_no::SysCallNo;

pub fn handle_syscall(syscall_n: usize, reg: &mut Registers) {
    let cap_ptr = reg.a0;
    let depth = reg.a1;
    let syscall_ret = if let Ok(inv_label) = InvLabel::try_from(reg.a2) {
        match syscall_n {
            n if n == SysCallNo::Print as usize => match inv_label {
                InvLabel::PutChar => {
                    let a0 = reg.a0;
                    putchar(a0 as u8);
                    Ok(None)
                }
                InvLabel::CNodeTraverse => {
                    if let Some(root_cnode) =
                        get_current_tcb_mut().root_cnode.as_ref().map(|slot| slot.cap_ref())
                    {
                        root_cnode.print_traverse();
                        Ok(None)
                    } else {
                        Err(kerr!(ErrKind::CapNotFound))
                    }
                }
                _ => Err(kerr!(ErrKind::UnknownSysCall)),
            },
            _ => {
                // Why don't you use "?"?
                handle_invocation(cap_ptr, depth, inv_label, syscall_n, reg)
            }
        }
    } else {
        println!("{}, {}", syscall_n, syscall_n);
        Err(kerr!(ErrKind::UnknownInvocation))
    };
    match syscall_ret {
        Err(e) => {
            println!("system call failed, {:?}", e);
            reg.a0 = e.e_kind as usize;
            reg.a1 = e.e_val as usize;
        }
        Ok(ret) => {
            reg.a0 = 0;
            if let Some(val) = ret {
                reg.a1 = val;
            }
        }
    };

    // increment pc
    reg.sepc += 4;
}

fn handle_invocation(
    cap_ptr: usize,
    depth: usize,
    inv_label: InvLabel,
    // TODO: Call or Send or Recv or NonBlocking Send or ..
    _syscall_n: usize,
    reg: &Registers,
) -> KernelResult<Option<usize>> {
    let current_tcb = get_current_tcb_mut();
    let ipc_buffer = current_tcb.ipc_buffer_ref();
    let mut root_cnode = current_tcb
        .root_cnode
        .as_ref()
        .ok_or(kerr!(ErrKind::CapNotFound))?
        .cap_ref()
        .replicate();
    // Hack
    let mut root_cnode_2 = root_cnode.replicate();

    let slot = root_cnode_2
        .lookup_entry_mut(cap_ptr, depth as u32)?
        .as_mut()
        .ok_or(kerr!(ErrKind::SlotIsEmpty))?;
    let cap_type = slot.get_cap_type()?;
    // TODO: Into Capability::invoke()
    match cap_type {
        CapabilityType::Untyped => {
            let dest_cnode_ptr = reg.a3;
            let index_and_depth = reg.a4;
            let user_size_and_num = reg.a5;
            let dest_depth = index_and_depth as u32;
            let index = index_and_depth >> 32;
            let user_size = user_size_and_num >> 32;
            let num = user_size_and_num as u32;
            let new_type = cap_try_from_u8(reg.a6 as u8)?;
            let dest_cnode_cap = root_cnode
                .lookup_entry_mut(dest_cnode_ptr, dest_depth)?
                .as_mut()
                .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                .as_capability::<CNode>()?
                .cap_ref_mut();
            let dest_cnode = dest_cnode_cap.get_writable(num, index as u32)?;
            let (src_cap, src_mdb) = slot.cap_and_mdb_ref_mut();
            src_cap.try_ref_mut_as::<Untyped>()?.invoke_retype(
                src_mdb,
                dest_cnode,
                user_size,
                num as usize,
                new_type,
            )?;
            // TODO: return user how match bit was used
            Ok(None)
        }
        CapabilityType::CNode => {
            let src_index = reg.a3;
            let src_depth = (reg.a4 >> 32) as u32;
            let dest_depth = reg.a4 as u32;
            let dest_index = reg.a5;

            let dest_root = slot.cap_ref_mut().try_ref_mut_as::<CNode>()?;
            let src_slot = root_cnode.lookup_entry_mut(src_index, src_depth)?;
            let src_entry = src_slot.as_mut().ok_or(kerr!(ErrKind::SlotIsEmpty))?;
            let dest_slot = dest_root.lookup_entry_mut(dest_index, dest_depth)?;
            if dest_slot.is_some() {
                Err(kerr!(ErrKind::NotEmptySlot))
            } else {
                // TODO: Whether this cap is derivable
                let raw_cap = src_entry.derive()?;
                let mut cap = raw_cap;
                if inv_label == InvLabel::CNodeMint {
                    let cap_val = reg.a6;
                    cap.set_cap_dep_val(cap_val);
                }
                let mut new_slot = CNodeEntry::new_with_rawcap(cap);
                if inv_label == InvLabel::CNodeMove {
                    new_slot.replace(src_entry);
                    *src_slot = None
                } else {
                    new_slot.insert(src_entry);
                }
                *dest_slot = Some(new_slot);
                Ok(None)
            }
        }
        CapabilityType::Tcb => {
            let tcb_cap = slot.cap_ref_mut().try_ref_mut_as::<ThreadControlBlock>()?;
            match inv_label {
                InvLabel::TcbConfigure => {
                    let cnode_ptr = reg.a3;
                    let cnode_depth = reg.a4 as u32;
                    let vspace_ptr = reg.a5;
                    let vspace_depth = reg.a6 as u32;
                    let (cnode_slot, vspace_slot) = root_cnode.lookup_two_entries_mut(
                        cnode_ptr,
                        cnode_depth,
                        vspace_ptr,
                        vspace_depth,
                    )?;
                    let cspace_slot = cnode_slot
                        .as_mut()
                        .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                        .as_capability::<CNode>()?;
                    let vspace_slot = vspace_slot
                        .as_mut()
                        .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                        .as_capability::<PageTable>()?;
                    tcb_cap.set_cspace(cspace_slot)?;
                    tcb_cap.set_vspace(vspace_slot)?;
                    Ok(None)
                }
                InvLabel::TcbWriteReg => {
                    let ipc_buffer = ipc_buffer.ok_or(kerr!(ErrKind::InvalidOperation))?;
                    let registers = ipc_buffer
                        .read_as::<Registers>()
                        .map_err(|_| kerr!(ErrKind::InvalidOperation))?;
                    tcb_cap.set_registers(registers);
                    Ok(None)
                }
                InvLabel::TcbSetIpcBuffer => {
                    let page_ptr = reg.a3;
                    let page_deph = reg.a4 as u32;
                    let page_cap = root_cnode
                        .lookup_entry_mut(page_ptr, page_deph)?
                        .as_mut()
                        .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                        .as_capability::<Page>()?;
                    tcb_cap.set_ipc_buffer(page_cap)?;
                    Ok(None)
                }
                InvLabel::TcbResume => {
                    tcb_cap.make_runnable();
                    Ok(None)
                }
                _ => Err(kerr!(ErrKind::UnknownInvocation)),
            }
        }
        CapabilityType::Notification => {
            // replicate is enough because send or wait operation will never change cap data.
            let mut notify_cap = slot
                .cap_ref_mut()
                .try_ref_mut_as::<Notification>()?
                .replicate();
            match inv_label {
                InvLabel::NotifySend => {
                    notify_cap.send();
                    Ok(None)
                }
                InvLabel::NotifyWait => {
                    if notify_cap.wait(current_tcb) {
                        require_schedule()
                    }
                    Ok(None)
                }
                _ => Err(kerr!(ErrKind::UnknownInvocation)),
            }
        }
        CapabilityType::EndPoint => {
            // replicate is enough because send or recv operation will never change cap data.
            let mut ep_cap = slot.cap_ref_mut().try_ref_mut_as::<Endpoint>()?.replicate();
            match inv_label {
                InvLabel::EpSend => {
                    if ep_cap.send(current_tcb) {
                        require_schedule()
                    }
                    Ok(None)
                }
                InvLabel::EpRecv => {
                    if ep_cap.recv(current_tcb) {
                        require_schedule()
                    }
                    Ok(None)
                }
                _ => Err(kerr!(ErrKind::UnknownInvocation)),
            }
        }
        CapabilityType::Page => {
            let page_cap = slot.cap_ref_mut().try_ref_mut_as::<Page>()?;
            match inv_label {
                InvLabel::PageMap => {
                    let page_table_ptr = reg.a3;
                    let page_table_depth = reg.a4 as u32;
                    let vaddr = reg.a5;
                    is_aligned(vaddr, PAGE_SIZE)
                        .then_some(())
                        .ok_or(kerr!(ErrKind::NotAligned, PAGE_SIZE as u16))?;
                    let root_page_table = root_cnode
                        .lookup_entry_mut(page_table_ptr, page_table_depth)?
                        .as_mut()
                        .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                        .cap_ref_mut()
                        .try_ref_mut_as::<PageTable>()?;
                    let flags = PAGE_U | get_user_flags(reg.a6);
                    page_cap.map(root_page_table, vaddr.into(), flags)?;
                    Ok(None)
                }
                InvLabel::PageUnMap => {
                    let page_table_ptr = reg.a3;
                    let page_table_depth = reg.a4 as u32;
                    let root_page_table = root_cnode
                        .lookup_entry_mut(page_table_ptr, page_table_depth)?
                        .as_mut()
                        .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                        .cap_ref_mut()
                        .try_ref_mut_as::<PageTable>()?;
                    page_cap.unmap(root_page_table)?;
                    Ok(None)
                }
                _ => Err(kerr!(ErrKind::UnknownInvocation)),
            }
        }
        CapabilityType::PageTable => {
            let page_table_cap = slot.cap_ref_mut().try_ref_mut_as::<PageTable>()?;
            match inv_label {
                InvLabel::PageTableMap => {
                    let page_table_ptr = reg.a3;
                    let page_table_depth = reg.a4 as u32;
                    let vaddr = reg.a5;
                    is_aligned(vaddr, PAGE_SIZE)
                        .then_some(())
                        .ok_or(kerr!(ErrKind::NotAligned, PAGE_SIZE as u16))?;
                    let root_page_table = root_cnode
                        .lookup_entry_mut(page_table_ptr, page_table_depth)?
                        .as_mut()
                        .ok_or(kerr!(ErrKind::SlotIsEmpty))?
                        .cap_ref_mut()
                        .try_ref_mut_as::<PageTable>()?;
                    let v = page_table_cap.map(root_page_table, vaddr.into())?;
                    Ok(Some(v))
                }
                InvLabel::PageTableUnMap => {
                    todo!()
                }
                InvLabel::PageTableMakeRoot => {
                    page_table_cap.make_as_root()?;
                    Ok(None)
                }
                _ => Err(kerr!(ErrKind::UnknownInvocation)),
            }
        }
    }
}
