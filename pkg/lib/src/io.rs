use crate::*;
use alloc::string::{String, ToString};
use alloc::vec;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    fn new() -> Self {
        Self
    }

    pub fn read_line(&self) -> String {
        // FIXME: allocate string
        let mut s = String::new();
        // FIXME: read from input buffer
        let mut buf = vec![0; 4];
        //s = sys_read(1, buf);
        //       - maybe char by char?
        loop {
            match sys_read(0, &mut buf) {
                Some(n) => {
                    if n > 0 {
                        let ch = String::from_utf8_lossy(&buf).to_string().remove(0);
                        match ch {
                            '\n' | '\r' => {
                                if s.is_empty() {
                                    self::print!("\n");
                                }
                                break;
                            }
                            '\x03' => {
                                //ctrl-C
                                s.clear();
                                self::print!("^C\n");
                                break;
                            }
                            '\x04' => {
                                //ctrl-D
                                s.clear();
                                self::print!("^D\n");
                                break;
                            }
                            '\x08' | '\x7f' => {
                                if !s.is_empty() {
                                    self::print!("\x08\x20\x08");
                                    s.pop();
                                }
                            }
                            _ => {
                                self::print!("{}", ch);
                                s.push(ch);
                            }
                        }
                    }
                }
                None => {
                    continue;
                }
            }
        }
        // FIXME: handle backspace / enter...
        // FIXME: return string
        s
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
