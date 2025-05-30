use crate::*;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

#[inline]
fn char_display_width(ch: char) -> usize {
    match ch as u32 {
        0x0000..=0x001F | 0x007F => 0,

        // 常见宽字符区段（CJK 统一表意、兼容、标点，全角、Hangul、Emoji …）
        0x1100..=0x115F |          // Hangul Jamo
        0x2329..=0x232A |          // 《 》
        0x2E80..=0xA4CF |          // CJK、部首、注音、拼音
        0xAC00..=0xD7A3 |          // Hangul Syllables
        0xF900..=0xFAFF |          // CJK 兼容
        0xFE10..=0xFE6F |          // CJK 标点
        0xFF01..=0xFF60 |          // 全角
        0xFFE0..=0xFFE6 |          // 全角 ¥ …
        0x1F000..=0x1FAFF => 2,    // 绝大多数 Emoji 区段

        _ => 1,                    // 其它按 1 列处理
    }
}

impl Stdin {
    fn new() -> Self {
        Self
    }

    pub fn read_line(&self) -> String {
        let mut line = String::new(); // 已解析好的字符
        let mut pending = Vec::<u8>::new(); // 正在拼合的 UTF-8 字节
        let mut one_byte = [0u8; 1]; // 每次读 1 字节

        loop {
            match sys_read(0, &mut one_byte) {
                Some(1) => {
                    let b = one_byte[0];
                    match b {
                        b'\n' | b'\r' => {
                            // 回车
                            self::print!("\n");
                            break;
                        }
                        0x03 => {
                            // Ctrl-C
                            line.clear();
                            self::print!("^C\n");
                            break;
                        }
                        0x04 => {
                            // Ctrl-D
                            line.clear();
                            self::print!("^D\n");
                            break;
                        }
                        0x08 | 0x7F => {
                            // Backspace / Delete
                            if let Some(ch) = line.pop() {
                                for _ in 0..char_display_width(ch) {
                                    self::print!("\x08\x20\x08");
                                }
                            }
                        }
                        _ => {
                            pending.push(b);
                            if let Ok(s) = core::str::from_utf8(&pending) {
                                if let Some(ch) = s.chars().next() {
                                    self::print!("{}", ch);
                                    line.push(ch);
                                    pending.clear();
                                }
                            } else if pending.len() >= 4 {
                                pending.clear();
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
        line
    }
}

impl Stdout {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(1, s.as_bytes());
    }
}

impl Stderr {
    fn new() -> Self {
        Self
    }

    pub fn write(&self, s: &str) {
        sys_write(2, s.as_bytes());
    }
}

pub fn stdin() -> Stdin {
    Stdin::new()
}

pub fn stdout() -> Stdout {
    Stdout::new()
}

pub fn stderr() -> Stderr {
    Stderr::new()
}
