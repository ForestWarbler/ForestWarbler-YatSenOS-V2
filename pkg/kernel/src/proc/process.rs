use super::*;
use crate::humanized_size;
use crate::memory::*;
use crate::proc::sync::*;
use crate::proc::vm::ProcessVm;
use crate::proc::vm::stack::*;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use chrono::offset;
use spin::*;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::page::PageRange;
use x86_64::structures::paging::*;

#[derive(Clone)]
pub struct Process {
    pid: ProcessId,
    inner: Arc<RwLock<ProcessInner>>,
}

pub struct ProcessInner {
    name: String,
    parent: Option<Weak<Process>>,
    children: Vec<Arc<Process>>,
    ticks_passed: usize,
    status: ProgramStatus,
    context: ProcessContext,
    exit_code: Option<isize>,
    proc_data: Option<ProcessData>,
    page_table: Option<PageTableContext>,
    proc_vm: Option<ProcessVm>,
}

impl Process {
    #[inline]
    pub fn pid(&self) -> ProcessId {
        self.pid
    }

    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<ProcessInner> {
        self.inner.write()
    }

    #[inline]
    pub fn read(&self) -> RwLockReadGuard<ProcessInner> {
        self.inner.read()
    }

    pub fn new(
        name: String,
        parent: Option<Weak<Process>>,
        proc_vm: Option<ProcessVm>,
        proc_data: Option<ProcessData>,
    ) -> Arc<Self> {
        let name = name.to_ascii_lowercase();

        // create context
        let pid = ProcessId::new();
        let page_table = proc_vm.as_ref().map(|vm| vm.page_table.clone_level_4());

        let inner = ProcessInner {
            name,
            parent,
            status: ProgramStatus::Ready,
            context: ProcessContext::default(),
            ticks_passed: 0,
            exit_code: None,
            children: Vec::new(),
            proc_vm: proc_vm,
            page_table: page_table,
            proc_data: Some(proc_data.unwrap_or_default()),
        };

        trace!("New process {}#{} created.", &inner.name, pid);

        // create process struct
        Arc::new(Self {
            pid,
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub fn kill(&self, ret: isize) {
        let mut inner = self.inner.write();

        debug!(
            "Killing process {}#{} with ret code: {}",
            inner.name(),
            self.pid,
            ret
        );

        inner.kill(ret);
    }

    pub fn alloc_init_stack(&self) -> VirtAddr {
        self.write().vm_mut().init_proc_stack(self.pid)
    }

    pub fn alloc_init_user_stack(&self) -> VirtAddr {
        self.write().vm_mut().init_user_proc_stack(self.pid)
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Process> {
        // FIXME: lock inner as write
        let mut inner = self.write();
        // FIXME: inner fork with parent weak ref
        let parent = Arc::downgrade(self);
        let child_inner = inner.fork(Some(parent));

        // FOR DBG: maybe print the child process info
        //          e.g. parent, name, pid, etc.

        // FIXME: make the arc of child
        let child = Arc::new(Process {
            pid: ProcessId::new(),
            inner: Arc::new(RwLock::new(child_inner)),
        });
        // FIXME: add child to current process's children list
        inner.children.push(child.clone());
        // FIXME: set fork ret value for parent with `context.set_rax`
        inner.context.set_rax(child.pid.0 as usize);
        drop(inner);
        child
    }
}

impl ProcessInner {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tick(&mut self) {
        self.ticks_passed += 1;
    }

    pub fn status(&self) -> ProgramStatus {
        self.status
    }

    pub fn pause(&mut self) {
        self.status = ProgramStatus::Ready;
    }

    pub fn resume(&mut self) {
        self.status = ProgramStatus::Running;
    }

    pub fn block(&mut self) {
        self.status = ProgramStatus::Blocked;
    }

    pub fn exit_code(&self) -> Option<isize> {
        self.exit_code
    }

    pub fn clone_page_table(&self) -> PageTableContext {
        self.page_table.as_ref().unwrap().clone_level_4()
    }

    pub fn is_ready(&self) -> bool {
        self.status == ProgramStatus::Ready
    }

    pub fn vm(&self) -> &ProcessVm {
        self.proc_vm.as_ref().unwrap()
    }

    pub fn vm_mut(&mut self) -> &mut ProcessVm {
        self.proc_vm.as_mut().unwrap()
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        self.vm_mut().handle_page_fault(addr)
    }

    /// Save the process's context
    /// mark the process as ready
    pub(super) fn save(&mut self, context: &ProcessContext) {
        // FIXME: save the process's context
        self.context.save(context);
        self.status = ProgramStatus::Ready;
    }

    /// Restore the process's context
    /// mark the process as running
    pub(super) fn restore(&mut self, context: &mut ProcessContext) {
        // FIXME: restore the process's context
        self.context.restore(context);
        self.status = ProgramStatus::Running;

        // FIXME: restore the process's page table
        self.vm().page_table.load();
    }

    pub fn parent(&self) -> Option<Arc<Process>> {
        self.parent.as_ref().and_then(|p| p.upgrade())
    }

    pub fn kill(&mut self, ret: isize) {
        // FIXME: set exit code
        self.exit_code = Some(ret);

        // FIXME: set status to dead
        self.status = ProgramStatus::Dead;

        // FIXME: take and drop unused resources
        self.page_table.take();
        self.proc_vm.take();
        self.proc_data.take();
    }

    pub fn init_stack_frame(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.context.init_stack_frame(entry, stack_top);
    }

    pub fn init_user_stack_frame(&mut self, entry: VirtAddr, stack_top: VirtAddr) {
        self.context.init_user_stack_frame(entry, stack_top);
    }

    pub fn load_elf(&mut self, elf: &ElfFile) {
        self.proc_vm.as_mut().unwrap().load_elf(elf);
    }

    pub fn fork(&self, parent: Option<Weak<Process>>) -> ProcessInner {
        // FIXME: fork the process virtual memory struct
        // FIXME: calculate the real stack offset
        // FIXME: update `rsp` in interrupt stack frame
        // Calculate the stack offset
        let stack_offset_count = self.children.len() as u64;
        let child_vm = self.proc_vm.as_ref().unwrap().fork(stack_offset_count);
        let child_page_table = child_vm.page_table.clone_level_4();

        let child_stack_bot = child_vm.stack.range().start.start_address().as_u64();
        let parent_stack_bot = self.vm().stack.range().start.start_address().as_u64();
        let offset = child_stack_bot - parent_stack_bot;

        // FIXME: set the return value 0 for child with `context.set_rax`
        let mut child_ctx: ProcessContext = self.context;
        child_ctx.set_rsp_offset(offset);
        child_ctx.set_rax(0);

        // FIXME: clone the process data struct
        let child_proc_data = self.proc_data.as_ref().map(|data| data.clone());

        // FIXME: construct the child process inner
        ProcessInner {
            name: self.name.clone(),
            parent,
            children: Vec::new(),
            ticks_passed: 0,
            status: ProgramStatus::Ready,
            context: child_ctx,
            exit_code: None,
            proc_data: self.proc_data.clone(),
            page_table: Some(child_page_table),
            proc_vm: Some(child_vm),
        }
        // NOTE: return inner because there's no pid record in inner
    }

    pub fn set_rax(&mut self, value: usize) {
        self.context.set_rax(value);
    }

    pub fn brk(&self, addr: Option<VirtAddr>) -> Option<VirtAddr> {
        self.proc_vm.as_ref().unwrap().brk(addr)
    }
}

impl core::ops::Deref for Process {
    type Target = Arc<RwLock<ProcessInner>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::Deref for ProcessInner {
    type Target = ProcessData;

    fn deref(&self) -> &Self::Target {
        self.proc_data
            .as_ref()
            .expect("Process data empty. The process may be killed.")
    }
}

impl core::ops::DerefMut for ProcessInner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.proc_data
            .as_mut()
            .expect("Process data empty. The process may be killed.")
    }
}

impl core::fmt::Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let inner = self.inner.read();
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("name", &inner.name)
            .field("parent", &inner.parent().map(|p| p.pid))
            .field("status", &inner.status)
            .field("ticks_passed", &inner.ticks_passed)
            .field("children", &inner.children.iter().map(|c| c.pid.0))
            .field("status", &inner.status)
            .field("context", &inner.context)
            .field("vm", &inner.proc_vm)
            .finish()
    }
}

impl core::fmt::Display for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let inner = self.inner.read();
        let (size, unit) = humanized_size(inner.proc_vm.as_ref().map_or(0, |vm| vm.memory_usage()));
        write!(
            f,
            " #{:-3} | #{:-3} | {:12} | {:7} | {:>5.1} {} | {:?}",
            self.pid.0,
            inner.parent().map(|p| p.pid.0).unwrap_or(0),
            inner.name,
            inner.ticks_passed,
            size,
            unit,
            inner.status
        )?;
        Ok(())
    }
}
