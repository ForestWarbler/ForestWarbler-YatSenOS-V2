#![no_std]
#![no_main]

#[macro_use]
extern crate log;

use core::arch::asm;
use ysos::interrupt;
use ysos_kernel as ysos;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    loop {
        info!("> ");
        let input = ysos::drivers::input::get_line();

        // This will cause a page fault
        // unsafe {
        //     let ptr = 0x7fffffffffff as *mut u32;
        //     *ptr = 42;
        // }

        match input.trim() {
            "exit" => break,
            _ => {
                info!("You said: {}", input);
                debug!("The counter value is {}", interrupt::clock::read_counter());
            }
        }
    }

    ysos::shutdown();
}
