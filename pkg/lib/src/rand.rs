use crate::syscall::*;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;

#[inline(always)]
pub fn new_rng() -> ChaCha20Rng {
    let seed = unsafe { sys_time() };
    ChaCha20Rng::seed_from_u64(seed)
}

#[inline(always)]
pub fn range(rng: &mut ChaCha20Rng, low: u64, high: u64) -> u64 {
    let span = high - low;
    (rng.next_u64() % span) + low
}
