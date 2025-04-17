use super::LocalApic;
use crate::interrupt::consts::*;
use bit_field::BitField;
use bitflags::bitflags;
use core::fmt::{Debug, Error, Formatter};
use core::ptr::{read_volatile, write_volatile};
use x86::cpuid::CpuId;

/// Default physical address of xAPIC
pub const LAPIC_ADDR: u64 = 0xFEE00000;

bitflags! {
    struct SpuriousFlags: u32 {
        const APIC_ENABLE = 1 << 8;
    }
}

bitflags! {
    struct LvtTimerFlags: u32 {
        const MASKED   = 1 << 16;
        const PERIODIC = 1 << 17;
    }
}

bitflags! {
    struct LvtLintFlags: u32 {
        const MASKED = 1 << 16;
    }
}

bitflags! {
    struct LvtPcintFlags: u32 {
        const MASKED = 1 << 16;
    }
}

bitflags! {
    struct LvtErrorFlags: u32 {
        const MASKED = 1 << 16;
    }
}

bitflags! {
    struct IcrFlags: u32 {
        const BCAST = 1 << 19;
        const INIT  = 5 << 8;
        const TMLV  = 1 << 15; // TM=1, LV=0
        const DS    = 1 << 12; // 传输状态
    }
}

const VECTOR_MASK: u32 = 0xFF;
pub struct XApic {
    addr: u64,
}

impl XApic {
    pub unsafe fn new(addr: u64) -> Self {
        XApic { addr }
    }

    unsafe fn read(&self, reg: u32) -> u32 {
        read_volatile((self.addr + reg as u64) as *const u32)
    }

    unsafe fn write(&mut self, reg: u32, value: u32) {
        write_volatile((self.addr + reg as u64) as *mut u32, value);
        self.read(0x20);
    }
}

impl LocalApic for XApic {
    /// If this type APIC is supported
    fn support() -> bool {
        // FIXME: Check CPUID to see if xAPIC is supported.
        let cpuid = CpuId::new();
        if let Some(finfo) = cpuid.get_feature_info() {
            finfo.has_apic()
        } else {
            false
        }
    }

    /// Initialize the xAPIC for the current CPU.
    fn cpu_init(&mut self) {
        unsafe {
            // FIXME: Enable local APIC; set spurious interrupt vector.
            let spurious_vector = Interrupts::IrqBase as u32 + Irq::Spurious as u32;
            let mut spiv = self.read(0xF0);
            spiv &= !VECTOR_MASK;
            spiv |= spurious_vector;
            spiv |= SpuriousFlags::APIC_ENABLE.bits();
            self.write(0xF0, spiv);

            // FIXME: The timer repeatedly counts down at bus frequency.
            let timer_vector = Interrupts::IrqBase as u32 + Irq::Timer as u32;
            let raw_timer = self.read(0x320);
            let base_timer = raw_timer & !VECTOR_MASK;
            let new_timer = base_timer | timer_vector;
            let mut timer_flags = LvtTimerFlags::from_bits_truncate(new_timer);
            timer_flags.remove(LvtTimerFlags::MASKED);
            timer_flags.insert(LvtTimerFlags::PERIODIC);
            self.write(0x320, (new_timer & VECTOR_MASK) | timer_flags.bits());
            self.write(0x3E0, 0b1011); // set Timer Divide to 1
            self.write(0x380, 0x20000); // set initial count to 0x20000

            // FIXME: Disable logical interrupt lines (LINT0, LINT1)
            self.write(0x350, LvtLintFlags::MASKED.bits());
            self.write(0x360, LvtLintFlags::MASKED.bits());

            // FIXME: Disable performance counter overflow interrupts (PCINT)
            self.write(0x340, LvtPcintFlags::MASKED.bits());

            // FIXME: Map error interrupt to IRQ_ERROR.
            let error_vector = Interrupts::IrqBase as u32 + Irq::Error as u32;
            let raw_error = self.read(0x370);
            let base_error = raw_error & !VECTOR_MASK;
            let new_error = base_error | error_vector;
            let mut error_flags = LvtErrorFlags::from_bits_truncate(new_error);
            error_flags.remove(LvtErrorFlags::MASKED);
            self.write(0x370, (new_error & VECTOR_MASK) | error_flags.bits());

            // FIXME: Clear error status register (requires back-to-back writes).
            self.write(0x280, 0);
            self.write(0x280, 0);

            // FIXME: Ack any outstanding interrupts.
            self.write(0xB0, 0);

            // FIXME: Send an Init Level De-Assert to synchronise arbitration ID's.
            self.write(0x310, 0);
            let icr_value = IcrFlags::BCAST.bits() | IcrFlags::INIT.bits() | IcrFlags::TMLV.bits();
            self.write(0x300, icr_value);
            while self.read(0x300) & IcrFlags::DS.bits() != 0 {}

            // FIXME: Enable interrupts on the APIC (but not on the processor).
            self.write(0x080, 0);
        }

        // NOTE: Try to use bitflags! macro to set the flags.
    }

    fn id(&self) -> u32 {
        // NOTE: Maybe you can handle regs like `0x0300` as a const.
        unsafe { self.read(0x0020) >> 24 }
    }

    fn version(&self) -> u32 {
        unsafe { self.read(0x0030) }
    }

    fn icr(&self) -> u64 {
        unsafe { (self.read(0x0310) as u64) << 32 | self.read(0x0300) as u64 }
    }

    fn set_icr(&mut self, value: u64) {
        unsafe {
            while self.read(0x0300).get_bit(12) {}
            self.write(0x0310, (value >> 32) as u32);
            self.write(0x0300, value as u32);
            while self.read(0x0300).get_bit(12) {}
        }
    }

    fn eoi(&mut self) {
        unsafe {
            self.write(0x00B0, 0);
        }
    }
}

impl Debug for XApic {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_struct("Xapic")
            .field("id", &self.id())
            .field("version", &self.version())
            .field("icr", &self.icr())
            .finish()
    }
}
