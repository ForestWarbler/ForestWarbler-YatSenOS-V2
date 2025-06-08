#![no_std]
#![no_main]

use lib::*;
extern crate alloc;
extern crate chrono;
extern crate lib;
use num_traits::float::FloatCore;

use alloc::{format, string::{String, ToString}, vec::Vec};
use chrono::{Datelike, NaiveDateTime, Timelike};

////////////////////////////////////////////////////////////////
// ANSI 样式常量
////////////////////////////////////////////////////////////////
const RESET:  &str = "\x1b[0m";
const BOLD:   &str = "\x1b[1m";
const RED:    &str = "\x1b[31m";
const GREEN:  &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE:   &str = "\x1b[34m";
const MAGENTA:&str = "\x1b[35m";
const CYAN:   &str = "\x1b[36m";
const WHITE:  &str = "\x1b[37m";

fn rainbow(s: &str) -> String {
    let step = 360.0 / (s.len().max(1) as f32);
    s.chars().enumerate().map(|(i, c)| {
        let h = step * i as f32;
        let (r, g, b) = hsv_to_rgb(h);
        format!("\x1b[38;2;{r};{g};{b}m{c}")
    }).collect::<String>() + RESET
}
fn hsv_to_rgb(h: f32) -> (u8, u8, u8) {
    let (h, s, v) = (h / 60.0 % 6.0, 1.0, 1.0);
    let f = h.fract();
    let (p, q, t) = ((1.0 - v) , (1.0 - f * v), (1.0 - (1.0 - f) * v));
    let (r, g, b) = match h as usize {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ((r*255.0) as u8, (g*255.0) as u8, (b*255.0) as u8)
}

fn prompt(cwd: &str) -> String {
    format!("{BOLD}{BLUE}{cwd}{RESET} {GREEN}>{RESET} ")
}

fn main() -> isize {
    let banner = [
        "┌───────────────────────────────────────────┐",
        "│             ⚡  fwsh  v0.1  ⚡            │",
        "│     type \u{201C}help\u{201D} to see built-ins …        │",
        "└───────────────────────────────────────────┘",
    ].iter()
     .map(|l| format!("{CYAN}{l}{RESET}"))
     .collect::<Vec<_>>()
     .join("\n");
    println!("{}", banner);

    let mut cwd = "/".to_string();

    loop {
        print!("{}", prompt(&cwd));

        let line_buf = io::stdin().read_line();
        let token: Vec<_> = line_buf.trim().split(' ').collect();

        match token[0] {
            "" => continue,

            "help" => {
                println!("{YELLOW}Built-ins:{RESET}");
                println!("  {GREEN}help{RESET}         – list help");
                println!("  {GREEN}echo <text>{RESET}  – output <text> as is");
                println!("  {GREEN}exit | quit{RESET} – exit shell");
                println!("  {GREEN}lsapp{RESET}        – list applications");
                println!("  {GREEN}ps{RESET}           – list processes");
                println!("  {GREEN}exec <app>{RESET}   – execute <app>");
                println!("  {GREEN}time{RESET}         – show current time");
                println!("  {GREEN}ls <dir>{RESET}     – list files");
                println!("  {GREEN}cwd{RESET}          – show cwd");
                println!("  {GREEN}cd <dir>{RESET}     – change dir");
            }

            "exit" | "quit" => {
                println!("\n{MAGENTA}► see you! ◄{RESET}\n");
                break;
            }

            "time" => {
                let ts_ms = sys_time() as i128;
                if ts_ms == 0 {
                    println!("{RED}Failed to get time{RESET}");
                    continue;
                }
                let beijing_ms = ts_ms + 8 * 60 * 60 * 1_000;
                let secs = (beijing_ms / 1_000) as i64;
                let nanos = ((beijing_ms % 1_000) as u32) * 1_000_000;

                if let Some(ndt) = NaiveDateTime::from_timestamp_opt(secs, nanos) {
                    println!("{CYAN}{:04}-{:02}-{:02} {:02}:{:02}:{:02}{RESET}",
                        ndt.year(), ndt.month(), ndt.day(),
                        ndt.hour(), ndt.minute(), ndt.second(),
                    );
                } else {
                    println!("{RED}(invalid timestamp){RESET}");
                }
            }

            "echo" => {
                let mut output = String::new();
                for i in 1..token.len() {
                    output.push_str(token[i]);
                    if i != token.len() - 1 { output.push(' '); }
                }
                println!("{}", output);
            }

            "lsapp" => {
                println!("{YELLOW}Applications:{RESET}");
                sys_list_app();
            }

            "ps" => {
                println!("{YELLOW}Processes:{RESET}");
                sys_stat();
            }

            "exec" => {
                if token.len() < 2 {
                    println!("{RED}Usage: exec <app_name>{RESET}");
                    continue;
                }
                let app_name = token[1];
                let ret = sys_wait_pid(sys_spawn(app_name));
                if ret == -1 {
                    println!("{RED}Failed to execute {}{RESET}", app_name);
                }
            }

            "ls" => {
                if token.len() < 2 {
                    sys_ls(&cwd);
                    continue;
                }
                let dir = token[1];
                let path = make_abs_path(&cwd, dir);
                if dir_exists(&path) {
                    sys_list_dir(&path);
                } else {
                    println!("{RED}Directory does not exist:{RESET} {}", path);
                }
            }

            "cwd" => {
                println!("{YELLOW}cwd:{RESET} {}", cwd);
            }

            "cd" => {
                let target = if token.len() < 2 || token[1].is_empty() { "/" } else { token[1] };
                let path = make_abs_path(&cwd, target);
                if dir_exists(&path) {
                    cwd = path;
                } else {
                    println!("{RED}Directory does not exist:{RESET} {}", path);
                }
            }

            "cat" => {
                if token.len() < 2 {
                    println!("{RED}Usage: cat <file_path>{RESET}");
                    continue;
                }
                sys_cat(token[1]);
            }

            unknown => {
                println!("{RED}Unknown command:{RESET} {}", unknown);
            }
        }
    }
    0
}

fn make_abs_path(cwd: &str, raw: &str) -> String {
    let mut path = if raw.starts_with('/') {
        raw.to_string()
    } else {
        if cwd == "/" { format!("/{raw}") } else { format!("{cwd}/{raw}") }
    };
    let mut stack: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {},
            ".."     => { stack.pop(); },
            p       => stack.push(p),
        }
    }
    path = if stack.is_empty() { "/".to_string() } else { format!("/{}", stack.join("/")) };
    path
}

entry!(main);
