use alloc::format;
use x86_64::{
    VirtAddr,
    structures::paging::{page::*, *},
};

use crate::{humanized_size, memory::*};
use elf::*;
use xmas_elf::ElfFile;

pub mod stack;

use self::stack::*;

use super::{PageTableContext, ProcessId};

type MapperRef<'a> = &'a mut OffsetPageTable<'static>;
type FrameAllocatorRef<'a> = &'a mut BootInfoFrameAllocator;

pub struct ProcessVm {
    // page table is shared by parent and child
    pub(super) page_table: PageTableContext,

    // stack is pre-process allocated
    pub(super) stack: Stack,
}

impl ProcessVm {
    pub fn new(page_table: PageTableContext) -> Self {
        Self {
            page_table,
            stack: Stack::empty(),
        }
    }

    pub fn init_kernel_vm(mut self) -> Self {
        // TODO: record kernel code usage
        self.stack = Stack::kstack();
        self
    }

    pub fn init_proc_stack(&mut self, pid: ProcessId) -> VirtAddr {
        // debug!("STACK_MAX_SIZE: {:#x}", STACK_MAX_SIZE);
        // FIXME: calculate the stack for pid
        debug!("PID: {:#x}", pid.0);
        let stack_bot_addr = STACK_INIT_BOT - (pid.0 as u64 - 1) * STACK_MAX_SIZE;
        let stack_top_addr = STACK_INIT_TOP - (pid.0 as u64 - 1) * STACK_MAX_SIZE;
        let frame_allocator = &mut *get_frame_alloc_for_sure();

        map_pages(
            stack_top_addr,
            STACK_DEF_PAGE,
            &mut self.page_table.mapper(),
            frame_allocator,
            true,
            false,
        );

        self.stack = Stack::new(
            Page::containing_address(VirtAddr::new(stack_top_addr)),
            STACK_DEF_PAGE,
        );

        VirtAddr::new(stack_top_addr)
    }

    pub fn init_user_proc_stack(&mut self, pid: ProcessId) -> VirtAddr {
        // debug!("STACK_MAX_SIZE: {:#x}", STACK_MAX_SIZE);
        // FIXME: calculate the stack for pid
        debug!("PID: {:#x}", pid.0);
        let stack_bot_addr = STACK_INIT_BOT;
        let stack_top_addr = STACK_INIT_TOP;
        let frame_allocator = &mut *get_frame_alloc_for_sure();

        map_pages(
            stack_top_addr,
            STACK_DEF_PAGE,
            &mut self.page_table.mapper(),
            frame_allocator,
            true,
            false,
        );

        self.stack = Stack::new(
            Page::containing_address(VirtAddr::new(stack_top_addr)),
            STACK_DEF_PAGE,
        );

        VirtAddr::new(stack_top_addr)
    }

    pub fn clean_user_stack(&mut self) {
        // clean user stack
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        let stack_top = self.stack.range().start;
        let stack_bot = self.stack.range().end;
        let page_count = self.stack.range().count();

        unmap_pages(
            stack_top.start_address().as_u64(),
            page_count as u64,
            mapper,
            alloc,
        )
        .expect("Unmap user stack failed.");
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.stack.handle_page_fault(addr, mapper, alloc)
    }

    pub(super) fn memory_usage(&self) -> u64 {
        self.stack.memory_usage()
    }

    pub fn load_elf(&mut self, elf: &ElfFile) {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.stack.init(mapper, alloc);

        // FIXME: load elf to process pagetable
        elf::load_elf(elf, *PHYSICAL_OFFSET.get().unwrap(), mapper, alloc, true);
    }

    pub fn fork(&self, stack_offset_count: u64) -> Self {
        // clone the page table context (see instructions)
        let owned_page_table = self.page_table.fork();

        let mapper = &mut owned_page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        Self {
            page_table: owned_page_table,
            stack: self.stack.fork(mapper, alloc, stack_offset_count),
        }
    }
}

impl core::fmt::Debug for ProcessVm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let (size, unit) = humanized_size(self.memory_usage());

        f.debug_struct("ProcessVm")
            .field("stack", &self.stack)
            .field("memory_usage", &format!("{} {}", size, unit))
            .field("page_table", &self.page_table)
            .finish()
    }
}
