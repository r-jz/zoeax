use crate::object::KObject;

use super::{Capability, CapabilityData, CapabilityType};

pub struct Irqs;

impl KObject for Irqs {}

pub type IrqControl = CapabilityData<Irqs>;

pub struct Irq;

impl KObject for Irq {}

pub type IrqHandler = CapabilityData<Irq>;

impl Capability for IrqControl {
    const CAP_TYPE: CapabilityType = CapabilityType::IrqControl;
    type KernelObject = Irqs;

    fn init_object(&mut self) {}
}

impl Capability for IrqHandler {
    const CAP_TYPE: CapabilityType = CapabilityType::IrqHandler;
    type KernelObject = Irq;

    fn init_object(&mut self) {}
}

impl IrqHandler {
    pub fn get_irq_number(&self) -> u64 {
        self.cap_dep_val & 0xfff
    }
}

