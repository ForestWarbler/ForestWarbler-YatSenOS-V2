#![cfg_attr(not(test), no_std)]
#![allow(dead_code, unused_imports)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod macros;

#[macro_use]
extern crate syscall_def;

#[macro_use]
pub mod io;
pub mod allocator;
pub extern crate alloc;

pub mod rand;
mod sync;
mod syscall;

use core::fmt::*;

pub use alloc::*;
pub use io::*;
pub use rand::*;
pub use sync::*;
pub use syscall::*;

use core::time::Duration;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => ($crate::_err(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! errln {
    () => ($crate::err!("\n"));
    ($($arg:tt)*) => ($crate::err!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    stdout().write(format!("{}", args).as_str());
}

#[doc(hidden)]
pub fn _err(args: Arguments) {
    stderr().write(format!("{}", args).as_str());
}

pub fn sleep(millisecs: u64) {
    let start = sys_time();
    let dur = millisecs;
    let mut current = start;
    while current - start < dur {
        current = sys_time();
    }
}

pub fn fork() -> u16 {
    sys_fork()
}

pub fn sys_ls(path: &str) {
    sys_list_dir(path);
}

pub fn dir_exists(path: &str) -> bool {
    sys_exists(path)
}

pub fn cat(path: &str) -> usize {
    sys_cat(path)
}