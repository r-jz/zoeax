use crate::{
    address::{KernelVAddress, PhysAddr},
    common::{ErrKind, KernelResult},
    kerr,
    object::{CNodeEntry, KObject},
};

pub use shared::cap_type::CapabilityType;
use shared::const_assert;

use core::mem;
use core::{marker::PhantomData, num::NonZeroU8};

pub mod cnode;
pub mod endpoint;
pub mod notification;
pub mod page_table;
pub mod tcb;
pub mod untyped;
pub mod irq;

const_assert!(mem::size_of::<CapabilityData<Something>>() == (mem::size_of::<u64>() * 2));

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
// Or K trait bound is not required.
pub struct CapabilityData<K: KObject> {
    cap_type: NonZeroU8,
    cap_right: u8,
    address_top: u16,
    address_bottom: u32,
    cap_dep_val: u64,
    _obj_type: PhantomData<K>,
}

impl<K: KObject> CapabilityData<K> {
    pub fn get_cap_type(&self) -> KernelResult<CapabilityType> {
        cap_try_from_u8(self.cap_type.get())
    }

    pub fn replicate(&self) -> Self {
        Self {
            cap_type: self.cap_type,
            cap_right: self.cap_right,
            address_top: self.address_top,
            address_bottom: self.address_bottom,
            cap_dep_val: self.cap_dep_val,
            _obj_type: self._obj_type,
        }
    }

    // TODO: u64 and usize
    pub fn set_cap_dep_val(&mut self, val: usize) {
        self.cap_dep_val = val as u64;
    }
}

/*
 * RawCapability[1]
 * | cap_type | cap_right | padding | address or none |
 * 64   5         5           6              48
 */

// This represents in slot capability.
#[derive(Debug)]
pub struct Something;
impl KObject for Something {}

pub type CapInSlot = CapabilityData<Something>;

impl CapInSlot {
    pub fn try_ref_mut_as<NK>(&mut self) -> KernelResult<&mut CapabilityData<NK>>
    where
        NK: KObject,
        CapabilityData<NK>: Capability,
    {
        let cap_type = self.get_cap_type()?;
        (CapabilityData::<NK>::CAP_TYPE == cap_type)
            .then_some(())
            .ok_or(kerr!(ErrKind::UnexpectedCapType))?;
        unsafe { Ok(self.unchecked_ref_mut_as()) }
    }

    pub fn try_ref_as<NK>(&self) -> KernelResult<&CapabilityData<NK>>
    where
        NK: KObject,
        CapabilityData<NK>: Capability,
    {
        let cap_type = self.get_cap_type()?;
        (CapabilityData::<NK>::CAP_TYPE == cap_type)
            .then_some(())
            .ok_or(kerr!(ErrKind::UnexpectedCapType))?;
        unsafe { Ok(self.unchecked_ref_as()) }
    }

    pub unsafe fn unchecked_ref_as<NK>(&self) -> &CapabilityData<NK>
    where
        NK: KObject,
        CapabilityData<NK>: Capability,
    {
        let ptr = self as *const Self as *const CapabilityData<NK>;
        ptr.as_ref().unwrap()
    }

    unsafe fn unchecked_ref_mut_as<NK>(&mut self) -> &mut CapabilityData<NK>
    where
        NK: KObject,
        CapabilityData<NK>: Capability,
    {
        let ptr = self as *mut Self as *mut CapabilityData<NK>;
        ptr.as_mut().unwrap()
    }
}

impl<K> From<CapabilityData<K>> for CapInSlot
where
    K: KObject,
    CapabilityData<K>: Capability,
{
    fn from(value: CapabilityData<K>) -> Self {
        Self {
            cap_type: value.cap_type,
            cap_dep_val: value.cap_dep_val,
            cap_right: value.cap_right,
            address_bottom: value.address_bottom,
            address_top: value.address_top,
            _obj_type: PhantomData,
        }
    }
}

impl<K: KObject> CapabilityData<K>
where
    CapabilityData<K>: Capability,
{
    pub fn init(address: KernelVAddress, user_size: usize) -> Self {
        let cap_type = Self::CAP_TYPE;
        let cap_dep_val = Self::create_cap_dep_val(address, user_size);
        Self::new(cap_type, address.into(), cap_dep_val as u64)
    }

    pub fn new(cap_type: CapabilityType, address: PhysAddr, cap_dep_val: u64) -> Self {
        let mut ret = Self {
            cap_type: NonZeroU8::new(cap_type as u8).unwrap(),
            cap_right: 0,
            address_top: 0,
            address_bottom: 0,
            cap_dep_val,
            _obj_type: PhantomData,
        };
        ret.set_address(address);
        ret
    }

    pub fn get_address(&self) -> PhysAddr {
        // TODO: u64 and usize
        let addr = (((self.address_top as u64) << 32) | self.address_bottom as u64) as usize;
        PhysAddr::new(addr)
    }

    pub fn set_address(&mut self, address: PhysAddr) {
        let address: usize = address.into();
        let address_top = ((address >> 32) & u16::MAX as usize) as u16;
        let address_bottom = (address & u32::MAX as usize) as u32;
        self.address_top = address_top;
        self.address_bottom = address_bottom
    }
}

pub fn cap_try_from_u8(val: u8) -> KernelResult<CapabilityType> {
    CapabilityType::try_from(val).map_err(|_| kerr!(ErrKind::UnknownCapType))
}

pub trait Capability
where
    Self: Sized,
{
    const CAP_TYPE: CapabilityType;
    type KernelObject: KObject;

    fn create_cap_dep_val(_addr: KernelVAddress, _user_size: usize) -> usize {
        0
    }
    fn get_object_size(_user_size: usize) -> usize {
        mem::size_of::<Self::KernelObject>()
    }
    fn can_be_retyped_from_device_memory() -> bool {
        false
    }
    fn derive(&self, _src_slot: &CNodeEntry<Something>) -> KernelResult<Self> {
        Err(kerr!(ErrKind::CanNotDerivable))
    }
    fn init_object(&mut self);
}
