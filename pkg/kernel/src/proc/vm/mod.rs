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

        map_range(
            stack_top_addr,
            STACK_DEF_PAGE,
            &mut self.page_table.mapper(),
            frame_allocator,
        );

        self.stack = Stack::new(
            Page::containing_address(VirtAddr::new(stack_top_addr)),
            STACK_DEF_PAGE,
        );

        VirtAddr::new(stack_top_addr)
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

        elf::map_range(STACK_INIT_BOT, STACK_DEF_PAGE, mapper, alloc);
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
