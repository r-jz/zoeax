use core::marker::PhantomData;
use core::mem;

use crate::address::KernelVAddress;
use crate::capability::PhysAddr;
use crate::capability::{Capability, CapabilityData, CapabilityType};
use crate::common::{align_up, ErrKind, KernelResult};
use crate::object::page_table::Page;
use crate::object::CNode;
use crate::object::Endpoint;
use crate::object::KObject;
use crate::object::ManagementDB;
use crate::object::Notification;
use crate::object::PageTable;
use crate::object::ThreadControlBlock;
use crate::object::Untyped;

use crate::kerr;

/*
 * RawCapability[0]
 * | 48 bit free_idx | 9 bit padd | 1 bit is_device | 6 bit block_size |
 * 64                                                                   0
 */

impl KObject for Untyped {}

pub type UntypedCap = CapabilityData<Untyped>;

impl Capability for UntypedCap {
    const CAP_TYPE: CapabilityType = CapabilityType::Untyped;
    type KernelObject = Untyped;

    fn create_cap_dep_val(addr: KernelVAddress, user_size: usize) -> usize {
        let is_device: usize = 0x00;
        let user_size = user_size.ilog2();
        let addr: PhysAddr = addr.into();
        let val: usize =
            (<PhysAddr as Into<usize>>::into(addr) << 16) | (is_device << 6) | user_size as usize;

        val
    }

    fn get_object_size(user_size: usize) -> usize {
        user_size
    }

    fn can_be_retyped_from_device_memory() -> bool {
        true
    }
    fn init_object(&mut self) {}
}

// impl fmt::Debug for UntypedCap {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "{:?}\n free index {:?}\nis_device {:?}\nblock_size {:?}",
//             self.cap_type,
//             self.get_free_index(),
//             self.is_device(),
//             self.block_size()
//         )
//     }
// }

impl UntypedCap {
    pub fn retype<K: KObject>(
        &mut self,
        user_size: usize,
        num: usize,
    ) -> KernelResult<CapGenerator<K>>
    where
        CapabilityData<K>: Capability,
    {
        // 1, can convert from device memory
        let is_device = self.is_device();
        if is_device {
            <CapabilityData<K>>::can_be_retyped_from_device_memory()
                .then_some(())
                .ok_or(kerr!(ErrKind::CanNotNewFromDeviceMemory))?
        }
        let block_size = self.block_size();
        let object_size = num * <CapabilityData<K>>::get_object_size(user_size);
        let align = mem::align_of::<<CapabilityData<K> as Capability>::KernelObject>();

        // 2, whether memory is enough or not
        let free_bytes = self.get_free_bytes();
        free_bytes
            .checked_sub(object_size)
            .ok_or(kerr!(ErrKind::NoMemory))?;
        // 3, create given type capabilities
        let free_idx_aligned = align_up(self.get_free_index().into(), align).into();
        let cap_generator = CapGenerator::<K>::new(num, free_idx_aligned, user_size, object_size);
        let new_free_address = cap_generator.end_address;
        // 4, update self information
        let v = Self::create_cap_dep_val(new_free_address, block_size);
        self.cap_dep_val = v as u64;
        if is_device {
            self.mark_is_device()
        }
        Ok(cap_generator)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn invoke_retype(
        &mut self,
        mdb: &mut ManagementDB,
        dest_cnode: &mut CNode,
        user_size: usize,
        num: usize,
        new_type: CapabilityType,
    ) -> KernelResult<()> {
        match new_type {
            CapabilityType::Tcb => {
                self.dispatch_retype::<ThreadControlBlock>(mdb, dest_cnode, user_size, num)
            }
            CapabilityType::CNode => {
                self.dispatch_retype::<CNode>(mdb, dest_cnode, user_size, num)
            }
            CapabilityType::EndPoint => {
                self.dispatch_retype::<Endpoint>(mdb, dest_cnode, user_size, num)
            }
            CapabilityType::Notification => {
                self.dispatch_retype::<Notification>(mdb, dest_cnode, user_size, num)
            }
            CapabilityType::PageTable => {
                self.dispatch_retype::<PageTable>(mdb, dest_cnode, user_size, num)
            }
            CapabilityType::Page => self.dispatch_retype::<Page>(mdb, dest_cnode, user_size, num),
            _ => Err(kerr!(ErrKind::UnknownCapType)),
        }
    }

    fn dispatch_retype<K: KObject>(
        &mut self,
        src_slot: &mut ManagementDB,
        dest_cnode: &mut CNode,
        user_size: usize,
        num: usize,
    ) -> KernelResult<()>
    where
        CapabilityData<K>: Capability,
    {
        let cap_gen = self.retype::<K>(user_size, num)?;
        for (i, mut cap) in cap_gen.enumerate() {
            cap.init_object();
            dest_cnode.insert_cap(src_slot, cap, i);
        }
        Ok(())
    }

    pub fn get_free_index(&self) -> KernelVAddress {
        let physadd = PhysAddr::new(self.cap_dep_val as usize >> 16);
        physadd.into()
    }

    pub fn is_device(&self) -> bool {
        ((&self.cap_dep_val >> 6) & 0x1) == 1
    }

    pub fn block_size(&self) -> usize {
        2_usize.pow((&self.cap_dep_val & 0x3f) as u32)
    }

    pub fn mark_is_device(&mut self) {
        self.cap_dep_val &= 0x3f
    }

    fn get_free_bytes(&self) -> usize {
        let start_address = KernelVAddress::from(self.get_address());
        let end_address = start_address.add(self.block_size());
        (end_address - self.get_free_index()).into()
    }
}

pub struct CapGenerator<K>
where
    K: KObject,
    CapabilityData<K>: Capability,
{
    num: usize,              // mutable
    address: KernelVAddress, // mutable
    user_size: usize,
    obj_size: usize,
    end_address: KernelVAddress,
    _phantom: PhantomData<K>,
}

impl<K> CapGenerator<K>
where
    K: KObject,
    CapabilityData<K>: Capability,
{
    pub fn new(
        num: usize,
        start_address: KernelVAddress,
        user_size: usize,
        obj_size: usize,
    ) -> Self {
        let end_address = KernelVAddress::new(
            <KernelVAddress as Into<usize>>::into(start_address) + obj_size * num,
        );
        Self {
            num,
            address: start_address,
            user_size,
            obj_size,
            _phantom: PhantomData,
            end_address,
        }
    }
}

impl<K> Iterator for CapGenerator<K>
where
    K: KObject,
    CapabilityData<K>: Capability,
{
    type Item = CapabilityData<K>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.num == 0 {
            None
        } else {
            let cap_dep_val = CapabilityData::<K>::create_cap_dep_val(self.address, self.user_size);
            let cap = CapabilityData::<K>::new(
                CapabilityData::<K>::CAP_TYPE,
                self.address.into(),
                cap_dep_val as u64,
            );
            self.address = self.address.add(self.obj_size);
            self.num -= 1;
            Some(cap)
        }
    }
}
