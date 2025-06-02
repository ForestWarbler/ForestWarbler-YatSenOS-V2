#![no_std]
#![no_main]

use lib::*;
extern crate lib;

const FISH_CNT: usize = 40;

static SEM_LT:   Semaphore = Semaphore::new(1);
static SEM_GT:   Semaphore = Semaphore::new(2);
static SEM_US:   Semaphore = Semaphore::new(3);
static SEM_DONE: Semaphore = Semaphore::new(4);

static PRINT: SpinLock = SpinLock::new();

fn putc(ch: u8) {
    PRINT.acquire();
    let _ = sys_write(1, &[ch]);
    PRINT.release();
}

fn proc_lt() -> ! {
    loop {
        SEM_LT.wait();
        putc(b'<');
        SEM_DONE.signal();
    }
}
fn proc_gt() -> ! {
    loop {
        SEM_GT.wait();
        putc(b'>');
        SEM_DONE.signal();
    }
}
fn proc_us() -> ! {
    loop {
        SEM_US.wait();
        putc(b'_');
        SEM_DONE.signal();
    }
}

fn main() -> isize {
    SEM_LT.init(0);
    SEM_GT.init(0);
    SEM_US.init(0);
    SEM_DONE.init(0);

    for child_fn in [proc_lt as fn() -> !, proc_gt, proc_us] {
        let pid = sys_fork();
        if pid == 0 {
            child_fn();
        }
    }

    let mut dir_left_first = true;
    for _ in 0..FISH_CNT {
        if dir_left_first {
            // print <><_
            SEM_LT.signal();   SEM_DONE.wait();
            SEM_GT.signal();   SEM_DONE.wait();
            SEM_LT.signal();   SEM_DONE.wait();
            SEM_US.signal();   SEM_DONE.wait();
        } else {
            // print ><>_
            SEM_GT.signal();   SEM_DONE.wait();
            SEM_LT.signal();   SEM_DONE.wait();
            SEM_GT.signal();   SEM_DONE.wait();
            SEM_US.signal();   SEM_DONE.wait();
        }
        dir_left_first = !dir_left_first;
    }

    PRINT.acquire();
    println!();
    PRINT.release();

    0
}

entry!(main);
