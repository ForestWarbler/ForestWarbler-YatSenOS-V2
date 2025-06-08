use core::sync::atomic::{AtomicU64, Ordering};

use alloc::sync::Arc;
use uefi::proto::debug;
use x86_64::{
    VirtAddr,
    structures::paging::{Page, mapper::UnmapError},
};

use super::{FrameAllocatorRef, MapperRef};

// user process runtime heap
// 0x100000000 bytes -> 4GiB
// from 0x0000_2000_0000_0000 to 0x0000_2000_ffff_fff8
pub const HEAP_START: u64 = 0x2000_0000_0000;
pub const HEAP_PAGES: u64 = 0x100000;
pub const HEAP_SIZE: u64 = HEAP_PAGES * crate::memory::PAGE_SIZE;
pub const HEAP_END: u64 = HEAP_START + HEAP_SIZE - 8;

/// User process runtime heap
///
/// always page aligned, the range is [base, end)
pub struct Heap {
    /// the base address of the heap
    ///
    /// immutable after initialization
    base: VirtAddr,

    /// the current end address of the heap
    ///
    /// use atomic to allow multiple threads to access the heap
    end: Arc<AtomicU64>,
}

impl Heap {
    pub fn empty() -> Self {
        Self {
            base: VirtAddr::new(HEAP_START),
            end: Arc::new(AtomicU64::new(HEAP_START)),
        }
    }

    pub fn fork(&self) -> Self {
        Self {
            base: self.base,
            end: self.end.clone(),
        }
    }

    pub fn brk(
        &self,
        new_end: Option<VirtAddr>,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> Option<VirtAddr> {
        // FIXME: if new_end is None, return the current end address
        if new_end.is_none() {
            debug!("Heap brk: returning current end address");
            return Some(VirtAddr::new(self.end.load(Ordering::Relaxed)));
        }

        // FIXME: check if the new_end is valid (in range [base, base + HEAP_SIZE])
        let new_end = new_end.unwrap();
        if new_end.as_u64() < self.base.as_u64()
            || new_end.as_u64() > self.base.as_u64() + HEAP_SIZE
        {
            debug!(
                "Heap brk: invalid range {:#x} (base: {:#x}, size: {:#x})",
                new_end.as_u64(),
                self.base.as_u64(),
                HEAP_SIZE
            );
            return None; // invalid range
        }

        // FIXME: calculate the difference between the current end and the new end
        let current_end = self.end.load(Ordering::Relaxed);
        let new_end = new_end.as_u64();
        if new_end == current_end {
            return Some(VirtAddr::new(current_end)); // no change needed
        }
        let diff = new_end - current_end;
        let diff_pages = diff / crate::memory::PAGE_SIZE;

        // NOTE: print the heap difference for debugging
        trace!(
            "Heap brk: current end = {:#x}, new end = {:#x}, diff = {:#x} ({} pages)",
            current_end, new_end, diff, diff_pages
        );

        // FIXME: do the actual mapping or unmapping
        if diff > 0 {
            // growing the heap
            let start_page = Page::containing_address(VirtAddr::new(current_end));
            let end_page = Page::containing_address(VirtAddr::new(new_end));
            let range = Page::range_inclusive(start_page, end_page);

            // map the new pages
            elf::map_range(range, mapper, alloc, true).unwrap();
        } else {
            // shrinking the heap
            let start_page = Page::containing_address(VirtAddr::new(new_end));
            let end_page = Page::containing_address(VirtAddr::new(current_end));
            let range = Page::range_inclusive(start_page, end_page);

            // unmap the old pages
            elf::unmap_range(range, mapper, alloc, true).unwrap();
        }

        // FIXME: update the end address
        self.end.store(new_end, Ordering::Relaxed);
        trace!("Heap end updated to {:#x}", new_end);
        Some(VirtAddr::new(new_end))
    }

    pub(super) fn clean_up(
        &self,
        mapper: MapperRef,
        dealloc: FrameAllocatorRef,
    ) -> Result<(), UnmapError> {
        if self.memory_usage() == 0 {
            return Ok(());
        }

        // FIXME: load the current end address and **reset it to base** (use `swap`)
        let current_end = self.end.swap(self.base.as_u64(), Ordering::Relaxed);

        // FIXME: unmap the heap pages
        let start_page = Page::containing_address(self.base);
        let end_page = Page::containing_address(VirtAddr::new(current_end));
        let range = Page::range_inclusive(start_page, end_page);

        elf::unmap_range(range, mapper, dealloc, true);

        Ok(())
    }

    pub fn memory_usage(&self) -> u64 {
        self.end.load(Ordering::Relaxed) - self.base.as_u64()
    }
}

impl core::fmt::Debug for Heap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Heap")
            .field("base", &format_args!("{:#x}", self.base.as_u64()))
            .field(
                "end",
                &format_args!("{:#x}", self.end.load(Ordering::Relaxed)),
            )
            .finish()
    }
}
