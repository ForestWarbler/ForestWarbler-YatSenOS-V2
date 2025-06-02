//! pkg/app/dinner/src/main.rs
#![no_std]
#![no_main]

use lib::rand::{new_rng, range};
use lib::*;

extern crate lib;

const N: usize = 5;
const EAT_TIMES: usize = 3;

static CHOPSTICK: [Semaphore; N] = semaphore_array!(0, 1, 2, 3, 4);
static PRINT: SpinLock = SpinLock::new();

#[inline(always)]
fn busy(n: u64) {
    for _ in 0..n {
        core::hint::spin_loop()
    }
}

fn philosopher(id: usize) {
    let l = id;
    let r = (id + 1) % N;
    let mut rng = new_rng();

    for k in 0..EAT_TIMES {
        // Test starvation
        // if id == 0 {
        //     busy(100000);
        // } else if id == 1 || id == 4 { /* Not thinking at all */
        // } else {
        //     busy(1000);
        // }

        if id & 1 == 0 {
            CHOPSTICK[l].wait();
            CHOPSTICK[r].wait();
        } else {
            CHOPSTICK[r].wait();
            CHOPSTICK[l].wait();
        }

        // Test deadlock
        // CHOPSTICK[l].wait();
        // busy(1000000);
        // println!("Philosopher {id} picked up left chopstick #{l}");
        // CHOPSTICK[r].wait();

        PRINT.acquire();
        println!("Philosopher {id} is eating #{k}");
        PRINT.release();

        busy(range(&mut rng, 500, 2000));

        // Test starvation
        // if id == 1 || id == 4 {
        //     busy(200); // Eat at the speed of light
        // } else {
        //     busy(3000); // Eat at a normal speed
        // }

        CHOPSTICK[l].signal();
        CHOPSTICK[r].signal();

        PRINT.acquire();
        println!("Philosopher {id} is thinking #{k}");
        PRINT.release();

        busy(range(&mut rng, 1000, 4000));
    }
}

fn main() -> isize {
    for c in 0..N {
        CHOPSTICK[c].init(1);
    }
    let mut pids = [0u16; N];

    for i in 0..N {
        let pid = sys_fork();
        if pid == 0 {
            philosopher(i);
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for &pid in &pids {
        sys_wait_pid(pid);
    }
    0
}

entry!(main);
