#![no_std]
#![no_main]

use lib::*;
extern crate alloc;
extern crate lib;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// ───────────────────────────────────────────
///  fwsh 0.1
///  Internal commands:
///     help         - list help
///     echo <text>  - output <text> as is
///     exit / quit  - exit shell
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
                println!("Built-ins: help, echo <text>, exit | quit");
            }

            "exit" | "quit" => {
                println!("bye");
                break;
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
