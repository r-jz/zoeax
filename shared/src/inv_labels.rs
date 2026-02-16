use crate::err_kind::ErrKind;

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvLabel {
    PutChar = 0,
    CNodeTraverse = 1,
    UntypedRetype = 2,
    TcbConfigure,
    TcbWriteReg,
    TcbResume,
    TcbSetIpcBuffer,
    NotifyWait,
    NotifySend,
    CNodeCopy,
    CNodeMint,
    CNodeMove,
    PageMap,
    PageUnMap,
    PageTableMap,
    PageTableUnMap,
    PageTableMakeRoot,
    EpSend,
    EpRecv,
    IRQControl,
    IRQHandlerAck,
    IRQHandlerSet,
}

impl TryFrom<usize> for InvLabel {
    type Error = ErrKind;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            inv if inv == Self::PutChar as usize => Ok(Self::PutChar),
            inv if inv == Self::CNodeTraverse as usize => Ok(Self::CNodeTraverse),
            inv if inv == Self::UntypedRetype as usize => Ok(Self::UntypedRetype),
            inv if inv == Self::TcbConfigure as usize => Ok(Self::TcbConfigure),
            inv if inv == Self::TcbWriteReg as usize => Ok(Self::TcbWriteReg),
            inv if inv == Self::TcbResume as usize => Ok(Self::TcbResume),
            inv if inv == Self::TcbSetIpcBuffer as usize => Ok(Self::TcbSetIpcBuffer),
            inv if inv == Self::NotifyWait as usize => Ok(Self::NotifyWait),
            inv if inv == Self::NotifySend as usize => Ok(Self::NotifySend),
            inv if inv == Self::CNodeCopy as usize => Ok(Self::CNodeCopy),
            inv if inv == Self::CNodeMint as usize => Ok(Self::CNodeMint),
            inv if inv == Self::CNodeMove as usize => Ok(Self::CNodeMove),
            inv if inv == Self::PageMap as usize => Ok(Self::PageMap),
            inv if inv == Self::PageUnMap as usize => Ok(Self::PageUnMap),
            inv if inv == Self::PageTableMap as usize => Ok(Self::PageTableMap),
            inv if inv == Self::PageTableUnMap as usize => Ok(Self::PageTableUnMap),
            inv if inv == Self::PageTableMakeRoot as usize => Ok(Self::PageTableMakeRoot),
            inv if inv == Self::EpSend as usize => Ok(Self::EpSend),
            inv if inv == Self::EpRecv as usize => Ok(Self::EpRecv),
            inv if inv == Self::IRQControl as usize => Ok(Self::IRQControl),
            inv if inv == Self::IRQHandlerAck as usize => Ok(Self::IRQHandlerAck),
            inv if inv == Self::IRQHandlerSet as usize => Ok(Self::IRQHandlerSet),
            _ => Err(ErrKind::UnknownInvocation),
        }
    }
}
