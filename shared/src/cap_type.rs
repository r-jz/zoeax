use crate::err_kind::ErrKind;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum CapabilityType {
    Untyped = 1,
    Tcb = 3,
    EndPoint = 5,
    CNode = 7,
    Notification = 9,
    IrqControl = 11,
    IrqHandler = 13,
    // Arch
    PageTable = 2,
    Page = 4,
}

impl TryFrom<u8> for CapabilityType {
    type Error = ErrKind;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == Self::Untyped as u8 => Ok(Self::Untyped),
            x if x == Self::Tcb as u8 => Ok(Self::Tcb),
            x if x == Self::EndPoint as u8 => Ok(Self::EndPoint),
            x if x == Self::CNode as u8 => Ok(Self::CNode),
            x if x == Self::Notification as u8 => Ok(Self::Notification),
            x if x == Self::IrqControl as u8 => Ok(Self::IrqControl),
            x if x == Self::IrqHandler as u8 => Ok(Self::IrqHandler),
            x if x == Self::Page as u8 => Ok(Self::Page),
            x if x == Self::PageTable as u8 => Ok(Self::PageTable),
            _ => Err(ErrKind::UnknownCapType),
        }
    }
}
