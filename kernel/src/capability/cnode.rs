use super::{CapInSlot, Capability, CapabilityData, CapabilityType, Something};
use crate::address::KernelVAddress;
use crate::common::{ErrKind, KernelResult};
use crate::object::{CNode, CNodeEntry, CSlot, KObject};
use crate::{kerr, print, println};

use core::{mem, ptr};

/*
 * RawCapability[0]
 * | padding |  radix  |
 * 63      32         0
 */
impl KObject for CNode {}

pub type CNodeCap = CapabilityData<CNode>;

impl Capability for CNodeCap {
    const CAP_TYPE: CapabilityType = CapabilityType::CNode;
    type KernelObject = CNode;
    fn create_cap_dep_val(_addr: KernelVAddress, user_size: usize) -> usize {
        user_size
    }

    fn get_object_size<'a>(user_size: usize) -> usize {
        2_usize.pow(user_size as u32) * mem::size_of::<CNodeEntry<Something>>()
    }
    fn derive(&self, _src_slot: &CNodeEntry<Something>) -> KernelResult<Self> {
        // unchecked
        Ok(Self { ..*self })
    }

    fn init_object(&mut self) {
        // TODO: Zero clear
    }
}

impl CNodeCap {
    fn capacity(&self) -> usize {
        2_usize.pow(self.radix())
    }

    pub fn get_cnode(&mut self) -> &mut [CSlot] {
        self.get_cnode_with_offset_mut(0, self.capacity())
    }

    fn get_cnode_with_offset_mut(&mut self, offset: u32, size: usize) -> &mut [CSlot] {
        let ptr: KernelVAddress = self.get_address().into();
        let ptr: *mut CSlot = ptr.into();
        unsafe { core::slice::from_raw_parts_mut(ptr.add(offset as usize), size) }
    }

    pub fn get_cnode_ref(&self) -> &[CSlot] {
        let ptr: KernelVAddress = self.get_address().into();
        let ptr: *const CSlot = ptr.into();
        unsafe { core::slice::from_raw_parts(ptr, self.capacity()) }
    }

    pub fn write_slot<C: Into<CapInSlot>>(&mut self, cap: C, index: usize) -> KernelResult<()> {
        if index >= self.capacity() {
            return Err(kerr!(ErrKind::OutOfMemory));
        }
        let cnode = self.get_cnode();
        let slot = &mut cnode[index];
        if slot.is_some() {
            return Err(kerr!(ErrKind::NotEmptySlot));
        }
        *slot = Some(CNodeEntry::new_with_rawcap(cap.into()));
        Ok(())
    }

    pub fn get_writable(&mut self, num: u32, index: u32) -> KernelResult<&mut CNode> {
        (2_usize.pow(self.radix()) > num as usize + index as usize)
            .then_some(())
            .ok_or(kerr!(ErrKind::InvalidOperation))?;
        let cnode = self.get_cnode_with_offset_mut(index, num as usize);
        cnode
            .iter()
            .all(|slot| slot.is_none())
            .then_some(())
            .ok_or(kerr!(ErrKind::NotEmptySlot))?;
        unsafe {
            let cnode = &mut cnode[0] as *mut CSlot as *mut CNode;
            Ok(cnode.as_mut().unwrap())
        }
    }

    pub fn lookup_two_entries_mut(
        &mut self,
        capptr: usize,
        depth_bits: u32,
        capptr2: usize,
        depth_bits2: u32,
    ) -> KernelResult<(&mut CSlot, &mut CSlot)> {
        let entry_1_ptr = { self.lookup_entry_mut(capptr, depth_bits)? as *mut CSlot };
        let entry_2 = self.lookup_entry_mut(capptr2, depth_bits2)?;
        // safety check not to return mutable reference of same memory area
        (!ptr::eq(entry_1_ptr, entry_2))
            .then_some(())
            .ok_or(kerr!(ErrKind::InvalidOperation))?;
        let entry_1 = unsafe { entry_1_ptr.as_mut().unwrap() };
        Ok((entry_1, entry_2))
    }

    pub fn lookup_entry_mut(&mut self, capptr: usize, depth_bits: u32) -> KernelResult<&mut CSlot> {
        let mut cnode_cap = self;
        let mut depth_bits = depth_bits;
        loop {
            let (next_cap, next_bits) = match cnode_cap._lookup_entry_mut(capptr, depth_bits)? {
                (val @ &mut None, _) => return Ok(val),
                (val, 0) => return Ok(val),
                (val, rem) => {
                    let cap_type = {
                        let entry = val.as_mut().unwrap();
                        entry.cap_ref().get_cap_type()?
                    };
                    if cap_type != CapabilityType::CNode {
                        return Ok(val);
                    }
                    let entry = val.as_mut().unwrap();
                    let cap = entry.cap_ref_mut().try_ref_mut_as().unwrap();
                    (cap, rem)
                }
            };
            cnode_cap = next_cap;
            depth_bits = next_bits;
        }
    }

    pub fn lookup_entry_mut_one_level(&mut self, capptr: usize) -> KernelResult<&mut CSlot> {
        self.lookup_entry_mut(capptr, self.radix())
    }

    fn _lookup_entry_mut(
        &mut self,
        capptr: usize,
        depth_bits: u32,
    ) -> KernelResult<(&mut CSlot, u32)> {
        let radix = self.radix();
        let remain_bits = depth_bits
            .checked_sub(radix)
            .ok_or(kerr!(ErrKind::OutOfMemory))?;
        let cnode = self.get_cnode();
        let offset = (capptr >> remain_bits) & ((1 << radix) - 1);
        let entry = &mut cnode[offset];
        Ok((entry, remain_bits))
    }

    fn radix(&self) -> u32 {
        self.cap_dep_val as u32
    }
    /// debug perpsoe
    pub fn print_traverse(&self) {
        self.print_level(0)
    }

    fn print_level(&self, level: usize) {
        let c_node = self.get_cnode_ref();
        for _ in 0..level {
            print!("  ");
        }
        print!("|");
        println!("level is {}, radix is {}", level, self.radix());
        for (i, slot) in c_node.iter().enumerate() {
            if let Some(ref entry) = slot {
                for _ in 0..level {
                    print!("   ");
                }
                print!("|");
                // TODO: Prity print
                print!("level is {}, index is {}, {:?}", level, i, entry);
                if let Ok(cap) = entry.cap_ref().try_ref_as::<CNode>() {
                    if cap.get_address() == self.get_address() {
                        println!("  # same cnode of current");
                    } else {
                        cap.print_level(level + 1)
                    };
                } else {
                    print!("\n");
                }
            }
        }
    }
}
