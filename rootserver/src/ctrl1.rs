#![no_std]
#![no_main]
#![feature(naked_functions)]

use core::{arch::naked_asm, panic::PanicInfo};

use libzoea::println;

mod boot_info;
mod elf;
mod rootserver;

extern "C" {
    static __stack_top: u8;
}

#[link_section = ".text.start"]
#[no_mangle]
#[unsafe(naked)]
extern "C" fn start() {
    unsafe {
        naked_asm!(
        "la sp, {stack_top}",
        "call main",
        "call exit",
        stack_top = sym __stack_top
        )
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:?}", info);
    loop {}
}

#[no_mangle]
fn exit() -> ! {
    panic!("exit")
}
