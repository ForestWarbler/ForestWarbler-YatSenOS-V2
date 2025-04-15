use super::consts::*;
use crate::memory::*;
use crate::proc::*;
use crate::utils::regs;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

pub unsafe fn register_idt(idt: &mut InterruptDescriptorTable) {
    idt[Interrupts::IrqBase as u8 + Irq::Timer as u8]
        .set_handler_fn(clock_handler)
        .set_stack_index(gdt::CLOCK_IST_INDEX);
}

// pub extern "x86-interrupt" fn clock_handler(_sf: InterruptStackFrame) {
//     x86_64::instructions::interrupts::without_interrupts(|| {
//         if inc_counter() % 0x10000 == 0 {
//             info!("Tick! @{}", read_counter());
//         }
//         super::ack();
//     });
// }

pub extern "C" fn clock(mut context: ProcessContext) {
    switch(&mut context);
    super::ack();
}
as_handler!(clock);

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn read_counter() -> u64 {
    // FIXME: load counter value
    COUNTER.load(Ordering::SeqCst)
}

#[inline]
pub fn inc_counter() -> u64 {
    // FIXME: read counter value and increase it
    COUNTER.fetch_add(1, Ordering::SeqCst)
}
