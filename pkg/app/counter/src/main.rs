#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const THREAD_COUNT: usize = 8;
static mut COUNTER: isize = 0;
static SPINLOCK: SpinLock = SpinLock::new();
static SEMAPHORE: Semaphore = Semaphore::new(1);

fn main() -> isize {
    let mut pids = [0u16; THREAD_COUNT];
    println!("==> Test with different modes: lock, sem, or default");
    let mode = io::stdin().read_line();

    if SEMAPHORE.init(1) != true {
        println!("Failed to initialize semaphore");
        return 0;
    }

    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            // do_counter_inc();
            if mode.trim() == "lock" {
                println!("thread #{} using lock", i);
                do_counter_inc_lock();
            } else if mode.trim() == "sem" {
                println!("thread #{} using semaphore", i);
                do_counter_inc_sem();
            } else {
                println!("thread #{} using default method", i);
                do_counter_inc_lock();
            }

            sys_exit(0);
        } else {
            pids[i] = pid; // only parent knows child's pid
        }
    }

    let cpid = sys_get_pid();
    println!("process #{} holds threads: {:?}", cpid, &pids);
    sys_stat();

    for i in 0..THREAD_COUNT {
        println!("#{} waiting for #{}...", cpid, pids[i]);
        sys_wait_pid(pids[i]);
    }

    println!("COUNTER result: {}", unsafe { COUNTER });

    0
}

fn do_counter_inc_lock() {
    for _ in 0..100 {
        // FIXME: protect the critical section
        SPINLOCK.acquire();
        inc_counter();
        SPINLOCK.release();
    }
}

fn do_counter_inc_sem() {
    for _ in 0..100 {
        // FIXME: protect the critical section
        SEMAPHORE.wait();
        inc_counter();
        SEMAPHORE.signal();
    }
}

/// Increment the counter
///
/// this function simulate a critical section by delay
/// DO NOT MODIFY THIS FUNCTION
fn inc_counter() {
    unsafe {
        delay();
        let mut val = COUNTER;
        delay();
        val += 1;
        delay();
        COUNTER = val;
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
fn delay() {
    for _ in 0..0x100 {
        core::hint::spin_loop();
    }
}

entry!(main);
