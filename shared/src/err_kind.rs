#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrKind {
    NoMemory = 1,
    PteNotFound,
    OutOfMemory,
    InvalidUserAddress,
    UnknownCapType,
    UnexpectedCapType,
    CanNotNewFromDeviceMemory,
    NoEnoughSlot,
    NotEmptySlot,
    SlotIsEmpty,
    VaddressAlreadyMapped,
    PageTableAlreadyMapped,
    PageTableNotMappedYet,
    PageAlreadyMapped,
    PageNotMappedYet,
    UnknownInvocation,
    CanNotDerivable,
    InvalidOperation,
    CapNotFound,
    NotAligned,
    UnknownSysCall,
    NotRootPageTable,
    IRQInvalid,
    IRQAlreadyActive,
    UnknownIRQ,
}

impl TryFrom<usize> for ErrKind {
    type Error = ();
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            e_val if e_val == ErrKind::NoMemory as usize => Ok(ErrKind::NoMemory),
            e_val if e_val == ErrKind::PteNotFound as usize => Ok(ErrKind::PteNotFound),
            e_val if e_val == ErrKind::OutOfMemory as usize => Ok(ErrKind::OutOfMemory),
            e_val if e_val == ErrKind::InvalidUserAddress as usize => {
                Ok(ErrKind::InvalidUserAddress)
            }
            e_val if e_val == ErrKind::UnknownCapType as usize => Ok(ErrKind::UnknownCapType),
            e_val if e_val == ErrKind::UnexpectedCapType as usize => Ok(ErrKind::UnexpectedCapType),
            e_val if e_val == ErrKind::CanNotNewFromDeviceMemory as usize => {
                Ok(ErrKind::CanNotNewFromDeviceMemory)
            }
            e_val if e_val == ErrKind::NoEnoughSlot as usize => Ok(ErrKind::NoEnoughSlot),
            e_val if e_val == ErrKind::NotEmptySlot as usize => Ok(ErrKind::NotEmptySlot),
            e_val if e_val == ErrKind::SlotIsEmpty as usize => Ok(ErrKind::SlotIsEmpty),
            e_val if e_val == ErrKind::VaddressAlreadyMapped as usize => {
                Ok(ErrKind::VaddressAlreadyMapped)
            }
            e_val if e_val == ErrKind::PageAlreadyMapped as usize => Ok(ErrKind::PageAlreadyMapped),
            e_val if e_val == ErrKind::PageNotMappedYet as usize => Ok(ErrKind::PageNotMappedYet),
            e_val if e_val == ErrKind::PageTableAlreadyMapped as usize => {
                Ok(ErrKind::PageTableAlreadyMapped)
            }
            e_val if e_val == ErrKind::PageTableNotMappedYet as usize => {
                Ok(ErrKind::PageTableNotMappedYet)
            }
            e_val if e_val == ErrKind::UnknownInvocation as usize => Ok(ErrKind::UnknownInvocation),
            e_val if e_val == ErrKind::CanNotDerivable as usize => Ok(ErrKind::CanNotDerivable),
            e_val if e_val == ErrKind::InvalidOperation as usize => Ok(ErrKind::InvalidOperation),
            e_val if e_val == ErrKind::CapNotFound as usize => Ok(ErrKind::CapNotFound),
            e_val if e_val == ErrKind::NotAligned as usize => Ok(ErrKind::NotAligned),
            _ => Err(()),
        }
    }
}
