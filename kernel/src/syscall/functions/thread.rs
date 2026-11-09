use crate::{device::pit, thread::scheduler::scheduler};

pub extern "C" fn sys_thread_yield() {
    scheduler().yield_cpu();
}

pub extern "C" fn sys_thread_wait(ms: u64) {
    pit::wait(ms as usize);
}

pub extern "C" fn sys_thread_exit() {
    scheduler().exit();
}

pub extern "C" fn sys_thread_get_id() -> u64 {
    scheduler().get_active_tid() as u64
}
