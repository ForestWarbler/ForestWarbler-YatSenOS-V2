use alloc::string::String;
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use alloc::vec::Vec;

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
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let key = pop_key();
        if key == b'\n' || key == b'\r' {
            break;
        } else {
            buf.push(key);
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}
