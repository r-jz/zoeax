use crate::object::KObject;

use super::{Capability, CapabilityData, CapabilityType};

pub struct IrqControl;

impl KObject for IrqControl {}

pub type IrqControlCap = CapabilityData<IrqControl>;

pub struct IrqHandler;

impl KObject for IrqHandler {}

pub type IrqHandlerCap = CapabilityData<IrqHandler>;

impl Capability for IrqControlCap {
    const CAP_TYPE: CapabilityType = CapabilityType::IrqControl;
    type KernelObject = IrqControl;

    fn init_object(&mut self) {}
}

impl Capability for IrqHandlerCap {
    const CAP_TYPE: CapabilityType = CapabilityType::IrqHandler;
    type KernelObject = IrqHandler;

    fn init_object(&mut self) {}
}

impl IrqControlCap {
    pub fn create() -> Self {
        // Address has no meaning because IrqControl is not backed by physical memory.
        Self::init(0.into(), 0)
    }
}

impl IrqHandlerCap {
    pub fn get_irq_number(&self) -> u64 {
        self.cap_dep_val & 0xfff
    }

    pub fn create(irq_number: u64) -> Self {
        let mut ret = Self::init(0.into(), 0);
        ret.cap_dep_val = irq_number & 0xfff;
        ret
    }
}
