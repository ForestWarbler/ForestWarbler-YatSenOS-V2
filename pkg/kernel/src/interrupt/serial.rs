use super::consts::*;
use crate::drivers::input;
use crate::drivers::serial::*;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Serial0 as u8].set_handler_fn(serial_handler);
}

pub extern "x86-interrupt" fn serial_handler(stack_frame: InterruptStackFrame) {
    receive();
    super::ack();
}

fn receive() {
    while let Some(byte) = get_serial_for_sure().receive() {
        input::push_key(byte);
    }
}
