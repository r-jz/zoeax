use crate::address::{KernelVAddress, PhysAddr, VirtAddr};
use crate::common::align_down;
use crate::memlayout::KERNEL_CODE_PFX;
use crate::object::page_table::{kernel_vm_root_mut, lv2table_addr, lv2table_mut};
use crate::object::PageTable;
use crate::println;

use core::ptr;

fn flag(leaf: bool) -> usize {
    if leaf {
        //    swdaguxwrv
        0b0011101111
    } else {
        //    swdaguxwrv
        0b0000000001
    }
}

extern "C" {
    static __text: u8;
    static __text_end: u8;
    static __data: u8;
    static __data_end: u8;
    static __rodata: u8;
    static __rodata_end: u8;
}

/// This function must be called only once at initialization.
/// After this function, we cannot access physical memory directly.
pub unsafe fn kernel_vm_init(free_ram_end_phys: usize) {
    let phyisical_start: usize = 0;
    let phyisical_end: usize = free_ram_end_phys;
    let kerenel_txt = ptr::addr_of!(__text) as usize;
    let kernel_data_end = ptr::addr_of!(__data_end) as usize;

    // first, mappin all physical memory
    let kernel_vm_root = kernel_vm_root_mut();
    let lv2_table = lv2table_mut();
    let step: usize = 2_usize.pow(9 + 9 + 9 + 12);
    for paddr in (phyisical_start..phyisical_end).step_by(step) {
        let paddr: PhysAddr = paddr.into();
        let vaddr: VirtAddr = KernelVAddress::from(paddr).into();
        let vpn = vaddr.get_vpn(3);
        let pte = &mut kernel_vm_root[vpn];
        pte.write(paddr, flag(true));
    }
    let vpn = VirtAddr::from(kerenel_txt).get_vpn(3);
    let lv2_addr = lv2table_addr();
    let pte = &mut kernel_vm_root[vpn];
    pte.write(lv2_addr.bit_and(!KERNEL_CODE_PFX), flag(false));

    // mapping elf;
    let step = 2_usize.pow(9 + 9 + 12);
    let elf_v_start = align_down(kerenel_txt, step);
    for vaddr in (elf_v_start..kernel_data_end).step_by(step) {
        let vaddr: VirtAddr = vaddr.into();
        let paddr: PhysAddr = vaddr.bit_and(!KERNEL_CODE_PFX).into();
        let vpn = vaddr.get_vpn(2);
        let pte = &mut lv2_table[vpn];
        pte.write(paddr, flag(true));
    }

    PageTable::activate_kernel_table();
    println!("root vm activation finished");
}
