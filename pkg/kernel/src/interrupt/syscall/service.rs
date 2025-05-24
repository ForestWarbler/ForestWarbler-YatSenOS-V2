use core::alloc::Layout;

use crate::proc::manager::get_process_manager;
use crate::proc::*;
use crate::utils::*;

use super::SyscallArgs;

pub fn spawn_process(args: &SyscallArgs) -> usize {
    // FIXME: get app name by args
    //       - core::str::from_utf8_unchecked
    //       - core::slice::from_raw_parts
    // FIXME: spawn the process by name
    // FIXME: handle spawn error, return 0 if failed
    // FIXME: return pid as usize
    let buf = unsafe { core::slice::from_raw_parts(args.arg0 as *const u8, args.arg1 as usize) };
    let name = unsafe { core::str::from_utf8_unchecked(buf) };
    let pid = crate::proc::spawn(name);
    if pid.is_none() {
        return 0;
    }

    pid.unwrap().0 as usize
}

pub fn sys_read(args: &SyscallArgs) -> usize {
    // FIXME: just like sys_write
    let fd = args.arg0 as u8;
    let buf = unsafe { core::slice::from_raw_parts_mut(args.arg1 as *mut u8, args.arg2 as usize) };

    crate::proc::read(fd, buf) as usize
}

pub fn sys_write(args: &SyscallArgs) -> usize {
    // FIXME: get buffer and fd by args
    //       - core::slice::from_raw_parts
    // FIXME: call proc::write -> isize
    // FIXME: return the result as usize
    let fd = args.arg0 as u8;
    let buf = unsafe { core::slice::from_raw_parts(args.arg1 as *const u8, args.arg2 as usize) };

    crate::proc::write(fd, buf) as usize
}

pub fn exit_process(args: &SyscallArgs, context: &mut ProcessContext) {
    // FIXME: exit process with retcode
    exit(args.arg0 as isize, context);
}

pub fn list_process() {
    // FIXME: list all processes
    print_process_list();
}

pub fn sys_allocate(args: &SyscallArgs) -> usize {
    let layout = unsafe { (args.arg0 as *const Layout).as_ref().unwrap() };

    if layout.size() == 0 {
        return 0;
    }

    let ret = crate::memory::user::USER_ALLOCATOR
        .lock()
        .allocate_first_fit(*layout);

    match ret {
        Ok(ptr) => ptr.as_ptr() as usize,
        Err(_) => 0,
    }
}

pub fn sys_deallocate(args: &SyscallArgs) {
    let layout = unsafe { (args.arg1 as *const Layout).as_ref().unwrap() };

    if args.arg0 == 0 || layout.size() == 0 {
        return;
    }

    let ptr = args.arg0 as *mut u8;

    unsafe {
        crate::memory::user::USER_ALLOCATOR
            .lock()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), *layout);
    }
}

pub fn sys_get_pid() -> usize {
    get_process_manager().current().pid().0 as usize
}

pub fn sys_wait_pid(args: &SyscallArgs) -> usize {
    let pid = args.arg0 as u16;
    let ret = get_process_manager().get_exit_code(&ProcessId(pid));
    match ret {
        Some(code) => code as usize,
        None => {
            return 20050615;
        }
    };

    ret.unwrap() as usize
}
