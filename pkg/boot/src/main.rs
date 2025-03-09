#![no_std]
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;

use core::fmt::Write;
use core::arch::asm;
use uefi::{Status, entry};
use uefi::prelude::*;

#[entry]
fn efi_main() -> Status {
    uefi::helpers::init().expect("Failed to initialize utilities");
    log::set_max_level(log::LevelFilter::Info);

    let std_num = 23336338;

    loop {
        info!("Hello World from UEFI bootloader! @ {}", std_num);

        for _ in 0..0x10000000 {
            unsafe {
                asm!("nop");
            }
        }
    }
}