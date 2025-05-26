use super::consts::*;
use crate::memory::*;
use crate::proc::*;
use crate::utils::regs;
use alloc::str;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

use uefi::runtime::get_time;

use chrono::offset::{FixedOffset, MappedLocalTime};
use chrono::prelude::*;

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

pub fn current_time_fixed() -> Option<DateTime<FixedOffset>> {
    let t = unsafe { uefi::runtime::get_time().ok()? };

    let date =
        NaiveDate::from_ymd_opt(t.year() as i32, u8::from(t.month()) as u32, t.day() as u32)?;

    let time = NaiveTime::from_hms_nano_opt(
        t.hour() as u32,
        t.minute() as u32,
        t.second() as u32,
        t.nanosecond() as u32,
    )?;

    let naive = date.and_time(time);

    let offset_sec = (t.time_zone().unwrap_or(0) as i32) * 60;
    let fixed = FixedOffset::east_opt(offset_sec)?;

    Some(fixed.from_utc_datetime(&naive))
}

pub fn current_time_string() -> Option<alloc::string::String> {
    current_time_fixed().map(|dt| {
        let d = dt.date_naive();
        let tm = dt.time();
        alloc::format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            d.year(),
            d.month(),
            d.day(),
            tm.hour(),
            tm.minute(),
            tm.second()
        )
    })
}
