use core::sync::atomic::{AtomicU16, Ordering};

use super::process::Process;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(pub u16);

impl ProcessId {
    pub fn new() -> Self {
        // FIXME: Get a unique PID
        static LAST_PID: AtomicU16 = AtomicU16::new(1);
        let pid = LAST_PID.fetch_add(1, Ordering::Relaxed);
        if pid == u16::MAX {
            LAST_PID.store(0, Ordering::Relaxed);
            ProcessId(0)
        } else {
            ProcessId(pid)
        }
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ProcessId> for u16 {
    fn from(pid: ProcessId) -> Self {
        pid.0
    }
}
