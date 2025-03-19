use alloc::string::String;
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;

type Key = u8;

lazy_static! {
    static ref INPUT_BUF: ArrayQueue<Key> = ArrayQueue::new(256);
}

#[inline]
pub fn push_key(key: Key) {
    if INPUT_BUF.push(key).is_err() {
        warn!("keyboard buffer full. Dropping key '{:?}'", key);
    }
}

#[inline]
pub fn try_pop_key() -> Option<Key> {
    INPUT_BUF.pop()
}

pub fn pop_key() -> Key {
    loop {
        if let Some(key) = try_pop_key() {
            return key;
        }
    }
}

pub fn get_line() -> String {
    let mut line = String::new();
    loop {
        let key = pop_key();
        if key == b'\n' || key == b'\r' {
            break;
        } else {
            line.push(key as char);
        }
    }
    line
}
