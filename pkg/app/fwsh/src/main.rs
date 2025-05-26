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

/// ───────────────────────────────────────────
///  fwsh 0.1
///  Internal commands:
///     help         - list help
///     echo <text>  - output <text> as is
///     exit / quit  - exit shell
///     lsapp        - list applications
///     ps           - list processes
///     exec <app>   - execute <app>
/// ───────────────────────────────────────────
fn main() -> isize {
    println!("► fwsh v0.1 — type \"help\" for help");

    let mut cwd = "/APP/";

    loop {
        print!("{} $ ", cwd);

        let line_buf = io::stdin().read_line();
        let token: Vec<_> = line_buf.trim().split(' ').collect();

        match token[0] {
            "" => continue,

            "help" => {
                println!("Built-ins: help, echo <text>, exit | quit, lsapp, ps, exec <app>");
            }

            "exit" | "quit" => {
                println!("bye");
                break;
            }

            "time" => {
                let ts_ms = sys_time() as i128;
                if ts_ms == 0 {
                    println!("Failed to get time");
                    continue;
                }

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
            }

            "echo" => {
                let mut output = String::new();
                for i in 1..token.len() {
                    output.push_str(token[i]);
                    if i != token.len() - 1 {
                        output.push(' ');
                    }
                }
                println!("{}", output);
            }

            "lsapp" => {
                println!("List of applications:");
                sys_list_app();
            }

            "ps" => {
                println!("List of processes:");
                sys_stat();
            }

            "exec" => {
                if token.len() < 2 {
                    println!("Usage: exec <app_name>");
                    continue;
                }
                let app_name = token[1];
                let ret = sys_wait_pid(sys_spawn(app_name));
                if ret == 0 {
                    println!("Failed to execute {}", app_name);
                }
            }

            unknown => {
                println!("Unknown command: {}", unknown);
            }
        }
    }

    0
}

entry!(main);
