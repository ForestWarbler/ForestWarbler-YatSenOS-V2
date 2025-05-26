#![no_std]
#![no_main]

use lib::*;
extern crate alloc;
extern crate chrono;
extern crate lib;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use chrono::{Datelike, NaiveDateTime, Timelike};

const MOD: u64 = 1000000007;

fn factorial(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1) % MOD
    }
}

fn main() -> isize {
    let ts_ms = sys_time() as i128;

    let beijing_ms = ts_ms + 8 * 60 * 60 * 1_000;

    let secs = (beijing_ms / 1_000) as i64;
    let sub_ms = (beijing_ms % 1_000) as u32;
    let nanos = sub_ms * 1_000_000;

    if let Some(ndt) = NaiveDateTime::from_timestamp_opt(secs, nanos) {
        println!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            ndt.year(),
            ndt.month(),
            ndt.day(),
            ndt.hour(),
            ndt.minute(),
            ndt.second(),
        );
    } else {
        println!("(invalid timestamp)");
    }

    println!("Sleeping for 2 second...");
    sleep(2000);

    let ts_ms = sys_time() as i128;

    let beijing_ms = ts_ms + 8 * 60 * 60 * 1_000;

    let secs = (beijing_ms / 1_000) as i64;
    let sub_ms = (beijing_ms % 1_000) as u32;
    let nanos = sub_ms * 1_000_000;

    if let Some(ndt) = NaiveDateTime::from_timestamp_opt(secs, nanos) {
        println!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            ndt.year(),
            ndt.month(),
            ndt.day(),
            ndt.hour(),
            ndt.minute(),
            ndt.second(),
        );
    } else {
        println!("(invalid timestamp)");
    }

    print!("Input n: ");

    let input = lib::stdin().read_line();

    // prase input as u64
    let n = input.parse::<u64>().unwrap();

    if n > 1000000 {
        println!("n must be less than 1000000");
        return 1;
    }

    // calculate factorial
    let result = factorial(n);

    // print system status
    sys_stat();

    // print result
    println!("The factorial of {} under modulo {} is {}.", n, MOD, result);

    0
}

entry!(main);
