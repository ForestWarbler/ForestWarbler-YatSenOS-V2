#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const CHILD_NUM: usize = 16;
const MSG_PER_ACTOR: usize = 10;
const BUF_SIZE: usize = 16;

#[derive(Clone, Copy)]
struct Message {
    pid: u16,
    value: u64,
}

static mut QUEUE: [Message; BUF_SIZE] = [Message { pid: 0, value: 0 }; BUF_SIZE];
// pointers for the queue
static mut IN: usize = 0;
static mut OUT: usize = 0;
// message count in the queue
static mut MSG_CNT: isize = 0;

// semaphore for synchronization
static EMPTY: Semaphore = Semaphore::new(1);
static FULL: Semaphore = Semaphore::new(2);
static MUTEX: Semaphore = Semaphore::new(3);

// spinlock for printing
static PRINT_LOCK: SpinLock = SpinLock::new();

fn producer(pid: u16) {
    for n in 0..MSG_PER_ACTOR {
        let msg = Message {
            pid,
            value: (pid as u64) * 100 + n as u64,
        };
        EMPTY.wait();
        MUTEX.wait();

        unsafe {
            QUEUE[IN] = msg;
            IN = (IN + 1) % BUF_SIZE;
            MSG_CNT += 1;
        }

        MUTEX.signal();
        FULL.signal();

        PRINT_LOCK.acquire();
        println!(
            "Producer #{pid} > produce {}, QUEUE_LEN={}",
            msg.value,
            unsafe { MSG_CNT }
        );
        PRINT_LOCK.release();
    }

    PRINT_LOCK.acquire();
    println!("Producer #{pid} finished.");
    PRINT_LOCK.release();
}

fn consumer(pid: u16) {
    for _ in 0..MSG_PER_ACTOR {
        FULL.wait();
        MUTEX.wait();

        let msg = unsafe {
            let m = QUEUE[OUT];
            OUT = (OUT + 1) % BUF_SIZE;
            MSG_CNT -= 1;
            m
        };

        MUTEX.signal();
        EMPTY.signal();

        PRINT_LOCK.acquire();
        println!(
            "Consumer #{pid} < consume {}, QUEUE_LEN={}",
            msg.value,
            unsafe { MSG_CNT }
        );
        PRINT_LOCK.release();
    }

    PRINT_LOCK.acquire();
    println!("Consumer #{pid} finished.");
    PRINT_LOCK.release();
}

fn main() -> isize {
    EMPTY.init(BUF_SIZE as usize);
    FULL.init(0);
    MUTEX.init(1);

    let mut pids = [0u16; CHILD_NUM];

    for i in 0..CHILD_NUM {
        let pid = sys_fork();
        if pid == 0 {
            let my_pid = sys_get_pid();
            if my_pid % 2 == 0 {
                producer(my_pid);
            } else {
                consumer(my_pid);
            }
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    println!("Parent #{}, Children = {:?}", sys_get_pid(), &pids);
    sys_stat();

    for &cpid in &pids {
        println!("Parent waiting for child #{cpid}...");
        sys_wait_pid(cpid);
    }

    println!("All children done, final QUEUE_LEN = {}", unsafe {
        MSG_CNT
    });
    0
}

entry!(main);
