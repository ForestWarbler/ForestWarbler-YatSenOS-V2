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
///     time         - show current time
///     ls <dir>     - list files in directory
///     cwd          - current working directory
///     cd <dir>     - change current working directory
/// ───────────────────────────────────────────
fn main() -> isize {
    println!("► fwsh v0.1 — type \"help\" for help");

    let mut cwd = "/".to_string();

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
                println!("\n === bye === \n");
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
                if ret == -1 {
                    println!("Failed to execute {}", app_name);
                }
            }

            "ls" => {
                if token.len() < 2 {
                    sys_ls(&cwd);
                    continue;
                }
                let dir = token[1];
                // Create absolute path
                let path = if dir.starts_with('/') {
                    dir.to_string()
                } else {
                    if cwd == "/" {
                        format!("/{dir}")
                    } else {
                        format!("{cwd}/{dir}")
                    }
                };
                // Format path
                let mut stack: Vec<&str> = Vec::new();
                for part in path.split('/') {
                    match part {
                        "" | "." => {}
                        ".." => {
                            stack.pop();
                        }
                        p => stack.push(p),
                    }
                }
                let path = if stack.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", stack.join("/"))
                };
                // Check if the path exists
                if dir_exists(&path) {
                    sys_list_dir(&path);
                } else {
                    println!("Directory does not exist: {}", path);
                }
            }

            "cwd" => {
                println!("Current working directory: {}", cwd);
            }

            "cd" => {
                // Get target directory
                let target = if token.len() < 2 || token[1].is_empty() {
                    "/"
                } else {
                    token[1]
                };

                // Create absolute path
                let mut path = if target.starts_with('/') {
                    target.to_string()
                } else {
                    if cwd == "/" {
                        format!("/{target}")
                    } else {
                        format!("{cwd}/{target}")
                    }
                };

                // Format path
                let mut stack: Vec<&str> = Vec::new();
                for part in path.split('/') {
                    match part {
                        "" | "." => {}
                        ".." => {
                            stack.pop();
                        }
                        p => stack.push(p),
                    }
                }
                path = if stack.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", stack.join("/"))
                };

                // Check if the path exists
                if dir_exists(&path) {
                    cwd = path;
                } else {
                    println!("Directory does not exist: {}", path);
                }
            }

            "cat" => {
                if token.len() < 2 {
                    println!("Usage: cat <file_path>");
                    continue;
                }
                let file_path = token[1];
                sys_cat(file_path);
            }

            unknown => {
                println!("Unknown command: {}", unknown);
            }
        }
    }

    0
}

entry!(main);
