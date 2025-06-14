use x86_64::{
    VirtAddr,
    structures::paging::{Page, mapper::MapToError, page::*},
};

use super::{FrameAllocatorRef, MapperRef};
use crate::proc;
use crate::proc::processor;
use core::ptr::copy_nonoverlapping;
use elf::*;
use x86_64::structures::paging::mapper::UnmapError;

// 0xffff_ff00_0000_0000 is the kernel's address space
pub const STACK_MAX: u64 = 0x4000_0000_0000;
pub const STACK_MAX_PAGES: u64 = 0x100000;
pub const STACK_MAX_SIZE: u64 = STACK_MAX_PAGES * crate::memory::PAGE_SIZE;
pub const STACK_START_MASK: u64 = !(STACK_MAX_SIZE - 1);
// [bot..0x2000_0000_0000..top..0x3fff_ffff_ffff]
// init stack
pub const STACK_DEF_BOT: u64 = STACK_MAX - STACK_MAX_SIZE;
pub const STACK_DEF_PAGE: u64 = 1;
pub const STACK_DEF_SIZE: u64 = STACK_DEF_PAGE * crate::memory::PAGE_SIZE;

pub const STACK_INIT_BOT: u64 = STACK_MAX - STACK_DEF_SIZE;
pub const STACK_INIT_TOP: u64 = STACK_MAX - 8;

const STACK_INIT_TOP_PAGE: Page<Size4KiB> = Page::containing_address(VirtAddr::new(STACK_INIT_TOP));

// [bot..0xffffff0100000000..top..0xffffff01ffffffff]
// kernel stack
pub const KSTACK_MAX: u64 = 0xffff_ff02_0000_0000;
pub const KSTACK_DEF_BOT: u64 = KSTACK_MAX - STACK_MAX_SIZE;
pub const KSTACK_DEF_PAGE: u64 = 3;
pub const KSTACK_DEF_SIZE: u64 = KSTACK_DEF_PAGE * crate::memory::PAGE_SIZE;

pub const KSTACK_INIT_BOT: u64 = KSTACK_MAX - KSTACK_DEF_SIZE;
pub const KSTACK_INIT_TOP: u64 = KSTACK_MAX - 8;

const KSTACK_INIT_PAGE: Page<Size4KiB> = Page::containing_address(VirtAddr::new(KSTACK_INIT_BOT));
const KSTACK_INIT_TOP_PAGE: Page<Size4KiB> =
    Page::containing_address(VirtAddr::new(KSTACK_INIT_TOP));

pub struct Stack {
    range: PageRange<Size4KiB>,
    usage: u64,
}

impl Stack {
    pub fn new(top: Page, size: u64) -> Self {
        Self {
            range: Page::range(top - size + 1, top + 1),
            usage: size,
        }
    }

    pub const fn empty() -> Self {
        Self {
            range: Page::range(STACK_INIT_TOP_PAGE, STACK_INIT_TOP_PAGE),
            usage: 0,
        }
    }

    pub const fn kstack() -> Self {
        Self {
            range: Page::range(KSTACK_INIT_PAGE, KSTACK_INIT_TOP_PAGE),
            usage: KSTACK_DEF_PAGE,
        }
    }

    pub fn init(&mut self, mapper: MapperRef, alloc: FrameAllocatorRef) {
        debug_assert!(self.usage == 0, "Stack is not empty.");

        self.range =
            elf::map_pages(STACK_INIT_BOT, STACK_DEF_PAGE, mapper, alloc, true, false).unwrap();
        self.usage = STACK_DEF_PAGE;
    }

    pub fn handle_page_fault(
        &mut self,
        addr: VirtAddr,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> bool {
        if !self.is_on_stack(addr) {
            return false;
        }

        if let Err(m) = self.grow_stack(addr, mapper, alloc) {
            error!("Grow stack failed: {:?}", m);
            return false;
        }

        true
    }

    fn is_on_stack(&self, addr: VirtAddr) -> bool {
        let addr = addr.as_u64();
        let cur_stack_bot = self.range.start.start_address().as_u64();
        trace!("Current stack bot: {:#x}", cur_stack_bot);
        trace!("Address to access: {:#x}", addr);
        addr & STACK_START_MASK == cur_stack_bot & STACK_START_MASK
    }

    fn grow_stack(
        &mut self,
        addr: VirtAddr,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> Result<(), MapToError<Size4KiB>> {
        let fault_addr = addr.as_u64();
        let cur_stack_bot = self.range.start.start_address().as_u64();
        let cur_stack_top = self.range.end.start_address().as_u64();

        debug!(
            "grow_stack: current stack range = [{:#x}..{:#x}), fault_addr = {:#x}",
            cur_stack_bot, cur_stack_top, fault_addr
        );

        if fault_addr < cur_stack_bot {
            let page_size = crate::memory::PAGE_SIZE;
            let delta = cur_stack_bot - fault_addr;
            let needed_pages = (delta + page_size - 1) / page_size;

            let new_stack_bot = cur_stack_bot
                .checked_sub(needed_pages * page_size)
                .ok_or(MapToError::FrameAllocationFailed)?;

            let user_access = (processor::get_pid() != proc::KERNEL_PID);

            if user_access {
                info!("Growing user stack from {:#x} to {:#x}", cur_stack_bot, new_stack_bot);
            } else {
                info!("Growing kernel stack from {:#x} to {:#x}", cur_stack_bot, new_stack_bot);
            }

            elf::map_pages(
                new_stack_bot,
                needed_pages,
                mapper,
                alloc,
                user_access,
                false,
            )?;

            self.range.start = Page::containing_address(VirtAddr::new(new_stack_bot));
            self.usage += needed_pages;
        }

        Ok(())
    }

    pub fn memory_usage(&self) -> u64 {
        self.usage * crate::memory::PAGE_SIZE
    }

    pub fn range(&self) -> &PageRange<Size4KiB> {
        &self.range
    }

    pub fn fork(
        &self,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
        mut stack_offset_count: u64,
    ) -> Self {
        let mut child_stack_top =
            self.range.start.start_address().as_u64() - stack_offset_count * STACK_MAX_SIZE;
        while elf::map_pages(child_stack_top, self.usage, mapper, alloc, true, false).is_err() {
            trace!(
                "Failed to map new stack on {:#x}, retrying...",
                child_stack_top
            );
            child_stack_top -= STACK_MAX_SIZE;
        }

        self.clone_range(
            self.range.start.start_address().as_u64(),
            child_stack_top,
            self.usage,
        );

        let start = Page::containing_address(VirtAddr::new(child_stack_top));
        Self {
            range: Page::range(start, start + self.usage),
            usage: self.usage,
        }
    }

    fn clone_range(&self, cur_addr: u64, dest_addr: u64, size: u64) {
        trace!("Clone range: {:#x} -> {:#x}", cur_addr, dest_addr);
        unsafe {
            copy_nonoverlapping::<u64>(
                cur_addr as *mut u64,
                dest_addr as *mut u64,
                (size * Size4KiB::SIZE / 8) as usize,
            );
        }
    }

    pub fn clean_up(
        &mut self,
        // following types are defined in
        //   `pkg/kernel/src/proc/vm/mod.rs`
        mapper: MapperRef,
        dealloc: FrameAllocatorRef,
    ) -> Result<(), UnmapError> {
        if self.usage == 0 {
            warn!("Stack is empty, no need to clean up.");
            return Ok(());
        }

        // FIXME: unmap stack pages with `elf::unmap_pagess`
        let stack_top = self.range.start.start_address().as_u64();
        elf::unmap_pages(stack_top, self.usage, mapper, dealloc);
        self.usage = 0;

        Ok(())
    }
}

impl core::fmt::Debug for Stack {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Stack")
            .field(
                "top",
                &format_args!("{:#x}", self.range.end.start_address().as_u64()),
            )
            .field(
                "bot",
                &format_args!("{:#x}", self.range.start.start_address().as_u64()),
            )
            .finish()
    }
}
