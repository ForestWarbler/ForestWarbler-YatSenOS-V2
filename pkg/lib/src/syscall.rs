use syscall_def::Syscall;

#[inline(always)]
pub fn sys_write(fd: u8, buf: &[u8]) -> Option<usize> {
    let ret = syscall!(
        Syscall::Write,
        fd as u64,
        buf.as_ptr() as u64,
        buf.len() as u64
    ) as isize;
    if ret.is_negative() {
        None
    } else {
        Some(ret as usize)
    }
}

#[inline(always)]
pub fn sys_read(fd: u8, buf: &mut [u8]) -> Option<usize> {
    let ret = syscall!(
        Syscall::Read,
        fd as u64,
        buf.as_ptr() as u64,
        buf.len() as u64
    ) as isize;
    if ret.is_negative() {
        None
    } else {
        Some(ret as usize)
    }
}

pub fn sys_time() -> u64 {
    syscall!(Syscall::Time) as u64
}

#[inline(always)]
pub fn sys_wait_pid(pid: u16) -> isize {
    // FIXME: try to get the return value for process
    //        loop until the process is finished
    let mut ret = -1;
    loop {
        ret = syscall!(Syscall::WaitPid, pid as u64) as isize;
        if ret != 20050615 {
            break;
        }
    }
    ret as isize
}

#[inline(always)]
pub fn sys_list_app() {
    syscall!(Syscall::ListApp);
}

#[inline(always)]
pub fn sys_stat() {
    syscall!(Syscall::Stat);
}

#[inline(always)]
pub fn sys_allocate(layout: &core::alloc::Layout) -> *mut u8 {
    syscall!(Syscall::Allocate, layout as *const _) as *mut u8
}

#[inline(always)]
pub fn sys_deallocate(ptr: *mut u8, layout: &core::alloc::Layout) -> usize {
    syscall!(Syscall::Deallocate, ptr, layout as *const _)
}

#[inline(always)]
pub fn sys_spawn(name: &str) -> u16 {
    syscall!(Syscall::Spawn, name.as_ptr() as u64, name.len() as u64) as u16
}

#[inline(always)]
pub fn sys_get_pid() -> u16 {
    syscall!(Syscall::GetPid) as u16
}

#[inline(always)]
pub fn sys_exit(code: isize) -> ! {
    syscall!(Syscall::Exit, code as u64);
    unreachable!("This process should be terminated by now.")
}

pub fn sys_fork() -> u16 {
    syscall!(Syscall::Fork) as u16
}

pub fn sys_new_sem(key: u32, init_value: usize) -> usize {
    syscall!(Syscall::Sem, 0, key, init_value)
}

pub fn sys_remove_sem(key: u32) -> usize {
    syscall!(Syscall::Sem, 1, key)
}

pub fn sys_sem_signal(key: u32) -> usize {
    syscall!(Syscall::Sem, 2, key)
}

pub fn sys_sem_wait(key: u32) -> usize {
    syscall!(Syscall::Sem, 3, key)
}

pub fn sys_list_dir(path: &str) -> usize {
    syscall!(Syscall::ListDir, path.as_ptr() as u64, path.len() as u64) as usize
}

pub fn sys_exists(path: &str) -> bool {
    syscall!(Syscall::Exists, path.as_ptr() as u64, path.len() as u64) != 0
}

pub fn sys_cat(path: &str) -> usize {
    syscall!(Syscall::Cat, path.as_ptr() as u64, path.len() as u64)
}
