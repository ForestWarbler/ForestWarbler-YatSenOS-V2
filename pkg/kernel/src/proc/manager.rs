use super::{processor::get_pid, *};
use crate::humanized_size;
use crate::memory::user::USER_ALLOCATOR;
use crate::memory::user::USER_HEAP_SIZE;
use crate::memory::{
    self, PAGE_SIZE,
    allocator::{ALLOCATOR, HEAP_SIZE},
    get_frame_alloc_for_sure,
};
use crate::proc::vm::ProcessVm;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::{collections::*, format};
use boot::{App, AppListRef};
use spin::{Mutex, RwLock};
use syscall_def::*;
use uefi::proto::debug;
use xmas_elf::ElfFile;

pub static PROCESS_MANAGER: spin::Once<ProcessManager> = spin::Once::new();

pub fn init(init: Arc<Process>, app_list: AppListRef) {
    // FIXME: set init process as Running
    init.write().resume();

    // FIXME: set processor's current pid to init's pid
    processor::set_pid(init.pid());
    let cur_pid = processor::get_pid();
    trace!("Current process: {:#?}", cur_pid);

    PROCESS_MANAGER.call_once(|| ProcessManager::new(init, app_list));
}

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER
        .get()
        .expect("Process Manager has not been initialized")
}

pub struct ProcessManager {
    processes: RwLock<BTreeMap<ProcessId, Arc<Process>>>,
    ready_queue: Mutex<VecDeque<ProcessId>>,
    app_list: AppListRef,
    wait_queue: Mutex<BTreeMap<ProcessId, BTreeSet<ProcessId>>>,
}

impl ProcessManager {
    pub fn new(init: Arc<Process>, app_list: AppListRef) -> Self {
        let mut processes = BTreeMap::new();
        let ready_queue = VecDeque::new();
        let pid = init.pid();
        let app_list = app_list;

        trace!("Init {:#?}", init);

        processes.insert(pid, init);
        Self {
            processes: RwLock::new(processes),
            ready_queue: Mutex::new(ready_queue),
            app_list,
            wait_queue: Mutex::new(BTreeMap::new()),
        }
    }

    #[inline]
    pub fn push_ready(&self, pid: ProcessId) {
        self.ready_queue.lock().push_back(pid);
    }

    #[inline]
    fn add_proc(&self, pid: ProcessId, proc: Arc<Process>) {
        self.processes.write().insert(pid, proc);
    }

    #[inline]
    fn get_proc(&self, pid: &ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(pid).cloned()
    }

    pub fn current(&self) -> Arc<Process> {
        self.get_proc(&processor::get_pid())
            .expect("No current process")
    }

    pub fn save_current(&self, context: &ProcessContext) {
        // FIXME: update current process's tick count
        let proc = self.current();
        proc.write().tick();
        // FIXME: save current process's context
        proc.write().save(context);
    }

    // pub fn switch_next(&self, context: &mut ProcessContext) -> ProcessId {
    //     // Fixed Version: Loop until a valid process is found
    //     loop {
    //         if let Some(next_pid) = self.ready_queue.lock().pop_front() {
    //             if let Some(next_proc) = self.get_proc(&next_pid) {
    //                 if next_proc.read().status() == ProgramStatus::Ready {
    //                     next_proc.write().restore(context);
    //                     processor::set_pid(next_pid);
    //                     return next_pid;
    //                 } else {
    //                     continue;
    //                 }
    //             }
    //         } else {
    //             warn!("No process in the ready queue.");
    //             return processor::get_pid();
    //         }
    //     }
    // }

    pub fn switch_next(&self, context: &mut ProcessContext) -> ProcessId {
        // FIXME: fetch the next process from ready queue
        let mut ready_queue = self.ready_queue.lock();
        if ready_queue.is_empty() {
            warn!("No process in the ready queue.");
            return ProcessId::new();
        }
        let cur_pid = processor::get_pid();
        let next_pid = ready_queue.pop_front().unwrap();
        let next_proc = self.get_proc(&next_pid).unwrap();
        // trace!("Switching to process {:#?}", next_proc);

        // FIXME: check if the next process is ready,
        //        continue to fetch if not ready
        if next_proc.read().status() != ProgramStatus::Ready {
            return get_pid();
        }

        // FIXME: restore next process's context
        next_proc.write().restore(context);

        // FIXME: update processor's current pid
        processor::set_pid(next_pid);

        // FIXME: return next process's pid
        next_pid
    }

    // pub fn spawn_kernel_thread(
    //     &self,
    //     entry: VirtAddr,
    //     name: String,
    //     proc_data: Option<ProcessData>,
    // ) -> ProcessId {
    //     let kproc = self.get_proc(&KERNEL_PID).unwrap();
    //     let page_table = kproc.read().clone_page_table();
    //     // debug!("Page table: {:#?}", page_table);
    //     // let proc_vm = Some(ProcessVm::new(page_table));
    //     let proc = Process::new(name, Some(Arc::downgrade(&kproc)), page_table, proc_data);

    //     // alloc stack for the new process base on pid
    //     let stack_top: VirtAddr = proc.alloc_init_stack();

    //     // FIXME: set the stack frame
    //     proc.write().init_stack_frame(entry, stack_top);

    //     // FIXME: add to process map
    //     let pid = proc.pid();
    //     self.add_proc(pid, Arc::clone(&proc));

    //     // FIXME: push to ready queue
    //     proc.write().pause();
    //     self.push_ready(pid);

    //     // FIXME: return new process pid
    //     pid
    // }

    pub fn spawn(
        &self,
        elf: &ElfFile,
        name: String,
        parent: Option<Weak<Process>>,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let kproc = self.get_proc(&KERNEL_PID).unwrap();
        let page_table = kproc.read().clone_page_table();
        // let proc_vm = Some(ProcessVm::new(page_table));
        let proc = Process::new(
            name,
            parent,
            Some(ProcessVm::new(page_table.clone_level_4())),
            proc_data,
        );
        let pid = proc.pid();

        let mut inner = proc.write();
        // FIXME: load elf to process pagetable
        inner.load_elf(elf);
        // FIXME: alloc new stack for process
        let stack_top: VirtAddr = inner.vm_mut().init_user_proc_stack(pid);
        info!(
            "spawn: pid={} entry={:#x} stack_top={:#x}",
            pid,
            elf.header.pt2.entry_point(),
            stack_top
        );
        inner.init_user_stack_frame(
            VirtAddr::new(elf.header.pt2.entry_point() as u64),
            stack_top,
        );
        // FIXME: mark process as ready
        inner.pause();
        drop(inner);

        trace!("New {:#?}", &proc);

        let pid = proc.pid();
        // FIXME: something like kernel thread
        self.add_proc(pid, Arc::clone(&proc));
        self.push_ready(pid);

        pid
    }

    pub fn fork(&self) -> u64 {
        // FIXME: get current process
        let parent = self.current();
        let parent_pid = parent.pid();
        trace!("Forking process: {}#{}", parent.read().name(), parent_pid);
        // FIXME: fork to get child
        let child = parent.fork();
        let child_pid = child.pid();
        trace!(
            "Forked child process: {}#{}",
            child.read().name(),
            child_pid
        );
        // FIXME: add child to process list
        self.add_proc(child_pid, Arc::clone(&child));

        parent.write().pause();
        child.write().pause();

        self.push_ready(parent_pid);
        self.push_ready(child_pid);

        // FOR DBG: maybe print the process ready queue?
        trace!("Queue  : {:?}\n", self.ready_queue.lock());

        child_pid.0 as u64
    }

    pub fn block(&self, pid: ProcessId) {
        if let Some(proc) = self.get_proc(&pid) {
            // FIXME: set the process as blocked
            proc.write().block();
        }
    }

    pub fn wait_pid(&self, pid: ProcessId) {
        let mut wait_queue = self.wait_queue.lock();
        // FIXME: push the current process to the wait queue
        //        `processor::get_pid()` is waiting for `pid`
        let cur_pid = processor::get_pid();
        wait_queue
            .entry(pid)
            .or_insert_with(BTreeSet::new)
            .insert(cur_pid);
    }

    pub fn wake_up(&self, pid: ProcessId, ret: Option<isize>) {
        if let Some(proc) = self.get_proc(&pid) {
            let mut inner = proc.write();
            if let Some(ret) = ret {
                // FIXME: set the return value of the process
                //        like `context.set_rax(ret as usize)`
                inner.set_rax(ret as usize);
            }
            // FIXME: set the process as ready
            // FIXME: push to ready queue

            inner.pause();
            self.push_ready(pid);
            trace!("Wake up process: {}#{}", inner.name(), pid);
        }
    }

    pub fn kill_current(&self, ret: isize) {
        self.kill(processor::get_pid(), ret);
    }

    pub fn handle_page_fault(&self, addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
        // FIXME: handle page fault
        if err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            warn!("Page fault: protection violation at {:#x}", addr.as_u64());
            return false;
        }

        if err_code.contains(PageFaultErrorCode::MALFORMED_TABLE) {
            error!(
                "Page fault: malformed page-table entry at {:#x}",
                addr.as_u64()
            );
            return false;
        }

        if err_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
            error!("Page fault: instruction fetch at {:#x}", addr.as_u64());
            return false;
        }

        if err_code.contains(PageFaultErrorCode::PROTECTION_KEY) {
            error!(
                "Page fault: protection key violation at {:#x}",
                addr.as_u64()
            );
            return false;
        }

        if err_code.contains(PageFaultErrorCode::SHADOW_STACK) {
            error!("Page fault: shadow stack violation at {:#x}", addr.as_u64());
            return false;
        }

        if err_code.contains(PageFaultErrorCode::SGX) {
            error!("Page fault: SGX violation at {:#x}", addr.as_u64());
            return false;
        }

        if err_code.contains(PageFaultErrorCode::RMP) {
            error!(
                "Page fault: RMP protection violation at {:#x}",
                addr.as_u64()
            );
            return false;
        }

        if !err_code.contains(PageFaultErrorCode::USER_MODE) {
            info!("Kernel page fault at {:#x}", addr.as_u64());
        }

        self.current().write().handle_page_fault(addr)
    }

    pub fn kill(&self, pid: ProcessId, ret: isize) {
        let proc = self.get_proc(&pid);

        if proc.is_none() {
            warn!("Process #{} not found.", pid);
            return;
        }

        let proc = proc.unwrap();

        if proc.read().status() == ProgramStatus::Dead {
            warn!("Process #{} is already dead.", pid);
            return;
        }

        trace!("Kill {:#?}", &proc);

        proc.kill(ret);

        if let Some(pids) = self.wait_queue.lock().remove(&pid) {
            for pid in pids {
                self.wake_up(pid, Some(ret));
            }
        }
    }

    pub fn print_process_list(&self) {
        let mut output = String::from("  PID | PPID | Process Name |  Ticks  | Status\n");

        self.processes
            .read()
            .values()
            .filter(|p| p.read().status() != ProgramStatus::Dead)
            .for_each(|p| output += format!("{}\n", p).as_str());

        // TODO: print memory usage of kernel heap
        let alloc = get_frame_alloc_for_sure();
        let frames_used = alloc.frames_used();
        let frames_total = alloc.frames_total();

        let used = frames_used as u64 * PAGE_SIZE as u64;
        let total = frames_total as u64 * PAGE_SIZE as u64;
        let (used_humanized, used_unit) = humanized_size(used);
        let (total_humanized, total_unit) = humanized_size(total);

        // info!("Physical Memory    : {:>7.*} {}", 3, size, unit);

        output += format!(
            "Memory Used: {:>7.*} {} / {:>7.*} {} ({:.2}%)\n",
            3,
            used_humanized,
            used_unit,
            3,
            total_humanized,
            total_unit,
            (used as f64 / total as f64) * 100.0
        )
        .as_str();

        // output += &Self::format_usage(
        //     "Memory",
        //     used as usize,
        //     total as usize,
        // );

        // Print Memory Usage of each process
        output += "\nProcess Memory Usage:\n";
        self.processes
            .read()
            .values()
            .filter(|p| p.read().status() != ProgramStatus::Dead)
            .for_each(|p| {
                output += format!("{}", p)
                .as_str();
            });

        output += format!("Queue  : {:?}\n", self.ready_queue.lock()).as_str();

        output += &processor::print_processors();

        print!("{}", output);
        drop(alloc);
    }

    fn format_usage(name: &str, used: usize, total: usize) -> String {
        let (used_float, used_unit) = humanized_size(used as u64);
        let (total_float, total_unit) = humanized_size(total as u64);

        format!(
            "{:<6} : {:>6.*} {:>3} / {:>6.*} {:>3} ({:>5.2}%)\n",
            name,
            2,
            used_float,
            used_unit,
            2,
            total_float,
            total_unit,
            used as f32 / total as f32 * 100.0
        )
    }

    pub fn get_exit_code(&self, pid: &ProcessId) -> Option<isize> {
        self.get_proc(pid).and_then(|proc| proc.read().exit_code())
    }

    pub fn get_proc_public(&self, pid: &ProcessId) -> Option<Arc<Process>> {
        self.get_proc(pid)
    }

    pub fn app_list(&self) -> AppListRef {
        self.app_list
    }

    pub fn read(&self, fd: u8, buf: &mut [u8]) -> isize {
        self.current().write().read(fd, buf)
    }

    pub fn write(&self, fd: u8, buf: &[u8]) -> isize {
        self.current().write().write(fd, buf)
    }
}
