mod context;
mod data;
pub mod manager;
mod paging;
mod pid;
mod process;
pub mod processor;
mod sync;

use crate::memory::PAGE_SIZE;
use manager::*;
use process::*;

use crate::alloc::string::ToString;
use alloc::string::String;
pub use context::ProcessContext;
pub use data::ProcessData;
pub use paging::PageTableContext;
pub use pid::ProcessId;
pub use sync::*;
use uefi::proto::debug;
pub mod vm;
use crate::drivers::filesystem::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use boot::BootInfo;
use storage::*;
use xmas_elf::ElfFile;

use crate::proc::vm::ProcessVm;
use x86_64::VirtAddr;
use x86_64::structures::idt::PageFaultErrorCode;
pub const KERNEL_PID: ProcessId = ProcessId(1);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramStatus {
    Running,
    Ready,
    Blocked,
    Dead,
}

/// init process manager
pub fn init(boot_info: &'static BootInfo) {
    let proc_vm = ProcessVm::new(PageTableContext::new()).init_kernel_vm(&boot_info.kernel_pages);

    trace!("Init kernel vm: {:#?}", proc_vm);

    // kernel process
    let kproc = Process::new(String::from("kernel_proc"), None, Some(proc_vm), None);

    let app_list = boot_info.loaded_apps.as_ref();
    manager::init(kproc, app_list);

    info!("Process Manager Initialized.");
}

pub fn switch(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXME: switch to the next process
        //      - save current process's context
        let manager = get_process_manager();

        //      - handle ready queue update
        let pid = crate::proc::processor::get_pid();
        if manager.current().read().status() == ProgramStatus::Running {
            manager.push_ready(pid);
        }

        //      - restore next process's context
        manager.save_current(context);
        manager.switch_next(context);
    });
}

// pub fn spawn_kernel_thread(entry: fn() -> !, name: String, data: Option<ProcessData>) -> ProcessId {
//     x86_64::instructions::interrupts::without_interrupts(|| {
//         let entry = VirtAddr::new(entry as usize as u64);
//         get_process_manager().spawn_kernel_thread(entry, name, data)
//     })
// }

pub fn print_process_list() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().print_process_list();
    })
}

pub fn env(key: &str) -> Option<String> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXME: get current process's environment variable
        get_process_manager().current().read().env(key)
    })
}

pub fn process_exit(ret: isize) -> ! {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().kill_current(ret);
    });

    loop {
        x86_64::instructions::hlt();
    }
}

pub fn handle_page_fault(addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().handle_page_fault(addr, err_code)
    })
}

pub fn list_app() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let app_list = get_process_manager().app_list();
        if app_list.is_none() {
            println!("[!] No app found in list!");
            return;
        }

        let apps = app_list
            .unwrap()
            .iter()
            .map(|app| app.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

        // TODO: print more information like size, entry point, etc.

        println!("[+] App list: {}", apps);
    });
}

pub fn spawn(name: &str) -> Option<ProcessId> {
    let app = x86_64::instructions::interrupts::without_interrupts(|| {
        let app_list = get_process_manager()
            .app_list()
            .expect("App list not found");
        app_list.iter().find(|&app| app.name.eq(name))
    })
    .expect("App not found");

    elf_spawn(name.to_string(), &app.elf)
}

// pub fn spawn(path: &str) -> Option<ProcessId> {
//     let mut file = get_rootfs().open_file(path).ok()?;

//     let mut elf_vec = Vec::<u8>::new();
//     file.read_all(&mut elf_vec).ok()?;

//     let exec_name = path
//         .trim_end_matches('/')
//         .rsplit('/')
//         .next()
//         .unwrap_or(path)
//         .to_string();

//     let elf = ElfFile::new(&elf_vec).ok()?;

//     elf_spawn(exec_name, &elf)
// }

pub fn elf_spawn(name: String, elf: &ElfFile) -> Option<ProcessId> {
    let pid = x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let process_name = name.to_lowercase();
        let parent = Arc::downgrade(&manager.current());
        let pid = manager.spawn(elf, name, Some(parent), None);

        debug!("Spawned process: {}#{}", process_name, pid);
        pid
    });

    Some(pid)
}

pub fn read(fd: u8, buf: &mut [u8]) -> isize {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().read(fd, buf))
}

pub fn write(fd: u8, buf: &[u8]) -> isize {
    x86_64::instructions::interrupts::without_interrupts(|| get_process_manager().write(fd, buf))
}

pub fn exit(ret: isize, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        // FIXME: implement this for ProcessManager
        manager.kill_current(ret);
        manager.switch_next(context);
    })
}

pub fn wait_pid(pid: u16, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let pid = ProcessId(pid);
        if let Some(ret) = manager.get_exit_code(&pid) {
            context.set_rax(ret as usize);
        } else {
            manager.wait_pid(pid);
            manager.save_current(context);
            manager.current().write().block();
            manager.switch_next(context);
        }
    })
}

pub fn fork(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        manager.save_current(context);
        let child_pid = manager.fork();
        manager.switch_next(context);
        // warn!("Forked process: {}#{}", manager.current().read().name(), child_pid);
    });
}

pub fn new_sem(key: u32, val: usize) -> usize {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let ret = manager.current().write().new_sem(key, val);
        ret as usize
    })
}

pub fn remove_sem(key: u32) -> usize {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let ret = manager.current().write().remove_sem(key);
        ret as usize
    })
}

pub fn sem_signal(key: u32, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let pid = processor::get_pid();
        let ret = manager.current().write().sem_signal(key);
        match ret {
            SemaphoreResult::Ok => context.set_rax(0),
            SemaphoreResult::NotExist => context.set_rax(1),
            SemaphoreResult::WakeUp(pid) => manager.wake_up(pid, Some(0)),
            _ => unreachable!(),
        }
    })
}

pub fn sem_wait(key: u32, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let pid = processor::get_pid();
        let ret = manager.current().write().sem_wait(key, pid);
        match ret {
            SemaphoreResult::Ok => context.set_rax(0),
            SemaphoreResult::NotExist => context.set_rax(1),
            SemaphoreResult::Block(pid) => {
                // FIXME: save, block it, then switch to next
                //        use `save_current` and `switch_next`
                manager.save_current(context);
                manager.current().write().block();
                manager.switch_next(context);
            }
            _ => unreachable!(),
        }
    })
}

pub fn brk(addr: Option<VirtAddr>) -> Option<VirtAddr> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // NOTE: `brk` does not need to get write lock
        get_process_manager().current().read().brk(addr)
    })
}

#[inline]
pub fn still_alive(pid: ProcessId) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let ret = get_process_manager().get_exit_code(&pid);
        if let None = ret { true } else { false }
    })
}
