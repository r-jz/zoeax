use crate::{
    address::{KernelVAddress, PhysAddr},
    capability::{irq::{IrqControl, IrqHandler}, CapInSlot, Capability, CapabilityData, Something},
    common::KernelResult,
    CapabilityType,
};
use core::{fmt::Debug, mem};
use shared::const_assert;

use super::{
    page_table::Page, Endpoint, KObject, Notification, PageTable, ThreadControlBlock, Untyped,
};

/*
 * ManagementDB[0]
 * |  prev node entry | padding |
 * 64                          0
 *       48               16
 * ManagementDB[1]
 * | next node entry  | padding |
 * 64                          0
 *       48               16
 */

const_assert!(mem::size_of::<CNodeEntry<Something>>() == mem::size_of::<CSlot>());

#[derive(Default, Debug)]
pub struct ManagementDB([usize; 2]);

impl ManagementDB {
    pub fn get_next(&mut self) -> Option<&mut CNodeEntry<Something>> {
        self.get_node(true)
    }

    pub fn set_next<C: KObject>(&mut self, next: &mut CNodeEntry<C>) {
        self.set_node(next, true)
    }

    pub fn get_prev(&mut self) -> Option<&mut CNodeEntry<Something>> {
        self.get_node(false)
    }

    pub fn set_prev<C: KObject>(&mut self, prev: &mut CNodeEntry<C>) {
        self.set_node(prev, false)
    }

    fn get_node(&mut self, is_next: bool) -> Option<&mut CNodeEntry<Something>> {
        let index = if is_next { 1 } else { 0 };
        let address = (self.0[index] >> 16) as *const CNodeEntry<Something>;
        if address.is_null() {
            None
        } else {
            let k_address: *mut CNodeEntry<Something> =
                KernelVAddress::from(PhysAddr::from(address)).into();
            unsafe { k_address.as_mut() }
        }
    }

    unsafe fn get_entry(&mut self) -> *mut CNodeEntry<Something> {
        let offset = mem::offset_of!(CNodeEntry<Something>, mdb);
        (self as *mut ManagementDB)
            .byte_sub(offset)
            .cast::<CNodeEntry<Something>>()
    }

    fn set_node<C: KObject>(&mut self, node: &mut CNodeEntry<C>, is_next: bool) {
        self._set_node(is_next, node);
        let parent = unsafe { self.get_entry().as_mut().unwrap() };
        node.mdb._set_node(!is_next, parent);
    }

    fn _set_node<C: KObject>(&mut self, is_next: bool, node: &CNodeEntry<C>) {
        let index = if is_next { 1 } else { 0 };
        self.0[index] &= 0xffff;
        self.0[index] |= (node.as_ref() as *const CNodeEntry<Something> as usize) << 16;
    }
}

pub type CSlot<T = Something> = Option<CNodeEntry<T>>;

#[derive(Debug)]
pub struct CNodeEntry<K: KObject>
where
    K: KObject,
{
    cap: CapabilityData<K>,
    mdb: ManagementDB,
}

impl CNodeEntry<Something> {
    pub fn as_capability<K>(&mut self) -> KernelResult<&mut CNodeEntry<K>>
    where
        K: KObject,
        CapabilityData<K>: Capability,
    {
        // whether cast is safe or
        self.cap.try_ref_mut_as::<K>()?;
        unsafe {
            let ptr = self as *mut Self as *mut CNodeEntry<K>;
            Ok(ptr.as_mut().unwrap())
        }
    }
    pub fn get_cap_type(&self) -> KernelResult<CapabilityType> {
        self.cap.get_cap_type()
    }

    fn dispatch_derive<K>(&self) -> KernelResult<CapInSlot>
        where 
            K: KObject,
            CapabilityData<K>: Capability
    {
        let cap = unsafe { self.cap.unchecked_ref_as::<K>() };
        cap.derive(self).map(Into::into)
    }

    pub fn derive(&self) -> KernelResult<CapInSlot> {
        match self.cap.get_cap_type()? {
            CapabilityType::Tcb => self.dispatch_derive::<ThreadControlBlock>(),
            CapabilityType::Untyped => self.dispatch_derive::<Untyped>(),
            CapabilityType::CNode => self.dispatch_derive::<CNode>(),
            CapabilityType::Notification => self.dispatch_derive::<Notification>(),
            CapabilityType::EndPoint => self.dispatch_derive::<Endpoint>(),
            CapabilityType::Page => self.dispatch_derive::<Page>(),
            CapabilityType::PageTable => self.dispatch_derive::<PageTable>(),
            CapabilityType::IrqControl => self.dispatch_derive::<IrqControl>(),
            CapabilityType::IrqHandler => self.dispatch_derive::<IrqHandler>(),
        }
    }
}

impl<K: KObject> CNodeEntry<K> {
    pub fn new_with_rawcap(cap: CapabilityData<K>) -> Self {
        Self {
            cap,
            mdb: ManagementDB([0; 2]),
        }
    }

    pub fn cap_and_mdb_ref_mut(&mut self) -> (&mut CapabilityData<K>, &mut ManagementDB) {
        (&mut self.cap, &mut self.mdb)
    }

    pub fn cap_ref(&self) -> &CapabilityData<K> {
        &self.cap
    }

    pub fn insert<C: KObject>(&mut self, parent: &mut CNodeEntry<C>) {
        if let Some(prev_next) = parent.get_next() {
            self.set_next(prev_next);
        };
        parent.set_next(self.as_mut())
    }

    pub fn replace<C: KObject>(&mut self, src: &mut CNodeEntry<C>) {
        if let Some(src_next) = src.get_next() {
            src_next.set_prev(self.as_mut());
            self.set_next(src_next);
        };
        if let Some(src_prev) = src.get_prev() {
            src_prev.set_next(self.as_mut());
            self.set_prev(src_prev);
        }
    }

    pub fn set_next<C: KObject>(&mut self, next: &mut CNodeEntry<C>) {
        self.mdb.set_next(next)
    }

    pub fn set_prev<C: KObject>(&mut self, prev: &mut CNodeEntry<C>) {
        self.mdb.set_prev(prev)
    }

    pub fn get_next(&mut self) -> Option<&mut CNodeEntry<Something>> {
        self.mdb.get_next()
    }

    pub fn get_prev(&mut self) -> Option<&mut CNodeEntry<Something>> {
        self.mdb.get_prev()
    }

    pub fn cap_ref_mut(&mut self) -> &mut CapabilityData<K> {
        &mut self.cap
    }
}

impl<K: KObject> AsRef<CNodeEntry<Something>> for CNodeEntry<K> {
    fn as_ref(&self) -> &CNodeEntry<Something> {
        unsafe {
            let ptr = self as *const CNodeEntry<K> as *const CNodeEntry<Something>;
            ptr.as_ref().unwrap()
        }
    }
}

impl<K: KObject> AsMut<CNodeEntry<Something>> for CNodeEntry<K> {
    fn as_mut(&mut self) -> &mut CNodeEntry<Something> {
        unsafe {
            let ptr = self as *mut CNodeEntry<K> as *mut CNodeEntry<Something>;
            ptr.as_mut().unwrap()
        }
    }
}

#[derive(Debug, Default)]
pub struct CNode;

impl CNode {
    pub fn new() -> Self {
        Self
    }

    pub fn insert_cap<C: Into<CapInSlot>>(
        &mut self,
        parent: &mut ManagementDB,
        cap: C,
        index: usize,
    ) {
        let root = (self as *mut Self).cast::<CNodeEntry<Something>>();
        let mut entry = CNodeEntry {
            cap: cap.into(),
            mdb: ManagementDB::default(),
        };
        if let Some(prev_next) = parent.get_next() {
            entry.set_next(prev_next);
        };
        parent.set_next(&mut entry);
        unsafe {
            *root.add(index) = entry;
        }
    }
}
