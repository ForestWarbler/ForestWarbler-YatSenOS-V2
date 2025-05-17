mod apic;
pub mod clock;
mod consts;
pub mod exceptions;
pub mod serial;
pub mod syscall;

use crate::interrupt::consts::*;
use crate::memory::physical_to_virtual;
use apic::*;
use x86_64::structures::idt::InterruptDescriptorTable;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            exceptions::register_idt(&mut idt);
            clock::register_idt(&mut idt);
            serial::register_idt(&mut idt);
            syscall::register_idt(&mut idt);
        }
        idt
    };
}

/// init interrupts system
pub fn init() {
    IDT.load();

    // FIXME: check and init APIC
    if XApic::support() {
        let mut lapic = unsafe { XApic::new(physical_to_virtual(LAPIC_ADDR)) };
        lapic.cpu_init();
    }

    // FIXME: enable serial irq with IO APIC (use enable_irq)
    enable_irq(Irq::Serial0 as u8, 0); // enable IRQ4 for CPU0

    info!("Interrupts Initialized.");
}

#[inline(always)]
pub fn enable_irq(irq: u8, cpuid: u8) {
    let mut ioapic = unsafe { IoApic::new(physical_to_virtual(IOAPIC_ADDR)) };
    ioapic.enable(irq, cpuid);
}

#[inline(always)]
pub fn ack() {
    let mut lapic = unsafe { XApic::new(physical_to_virtual(LAPIC_ADDR)) };
    lapic.eoi();
}
