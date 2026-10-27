/*
 * Module: user_api
 *
 * Description: All system calls available to user programs are defined in this module.
 *
 * Author: Stefan Lankes, RWTH Aachen University
 *         Licensed under the Apache License, Version 2.0 or MIT license, at your option.
 *
 *         Michael Schoettner, Heinrich Heine University Duesseldorf, 14.09.2023
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 15.10.2025
 */

use core::arch::asm;

use crate::device::key::KeyEvent;

/// System call numbers available to user programs.
#[repr(u64)]
pub enum SyscallFunction {
    HelloWorld,
    ThreadYield,
    ThreadWait,
    ThreadExit,
    ThreadGetId,
    GetSystemTime,
    Print,
    GetKeyEvent,
    NumSyscalls, // Last entry to count number of syscalls
}

/// Test system call printing "Hello, World!" to the serial console.
pub fn usr_hello_world() {
    unsafe {
        syscall0(SyscallFunction::HelloWorld);
    }
}

/// Yields the CPU and switches to the next thread.
pub fn usr_thread_yield() {
    unsafe {
        syscall0(SyscallFunction::ThreadYield);
    }
}

/// Yields the CPU for the given time.
pub fn usr_thread_wait(ms: usize) {
    unsafe {
        syscall1(SyscallFunction::ThreadWait, ms as u64);
    }
}

/// Terminates the current thread.
pub fn usr_thread_exit() {
    unsafe {
        syscall0(SyscallFunction::ThreadExit);
    }
}

/// Returns the ID of the current thread.
pub fn usr_thread_get_id() -> usize {
    unsafe { syscall0(SyscallFunction::ThreadGetId) as usize }
}

/// Returns the current system time in milliseconds.
pub fn usr_get_system_time() -> usize {
    unsafe { syscall0(SyscallFunction::GetSystemTime) as usize }
}

pub fn usr_print(msg: &str) {
    unsafe {
        syscall2(
            SyscallFunction::Print,
            msg.as_ptr() as u64,
            msg.len() as u64,
        );
    }
}

pub fn usr_get_key_event() -> Option<KeyEvent> {
    let mut key_event = KeyEvent::new();
    let ptr = core::ptr::from_mut(&mut key_event);

    let available = unsafe { syscall1(SyscallFunction::GetKeyEvent, ptr as u64) };

    if available == 0 {
        return None;
    }

    Some(key_event)
}

/// Perform a system call with 0 parameters.
#[inline(always)]
pub unsafe fn syscall0(syscall: SyscallFunction) -> u64 {
    let mut ret: u64;
    unsafe {
        asm!(
        "int 0x80",
        inlateout("rax") syscall as u64 => ret, // after the syscall, rax contains the return value
        options(preserves_flags, nostack)
        );
    }
    ret
}

/// Perform a system call with 1 parameter.
#[inline(always)]
pub unsafe fn syscall1(syscall: SyscallFunction, arg1: u64) -> u64 {
    let mut ret: u64;
    unsafe {
        asm!(
        "int 0x80",
        inlateout("rax") syscall as u64 => ret, // after the syscall, rax contains the return value
        in("rdi") arg1,
        options(preserves_flags, nostack)
        );
    }
    ret
}

/// Perform a system call with 2 parameters.
#[inline(always)]
pub unsafe fn syscall2(syscall: SyscallFunction, arg1: u64, arg2: u64) -> u64 {
    let mut ret: u64;
    unsafe {
        asm!(
        "int 0x80",
        inlateout("rax") syscall as u64 => ret, // after the syscall, rax contains the return value
        in("rdi") arg1, in("rsi") arg2,
        options(preserves_flags, nostack)
        );
    }
    ret
}
