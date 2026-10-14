/*
 * Contains functions to create, start, switch and end threads.
 *
 * Author: Michael Schoettner, Heinrich Heine University Duesseldorf, 2023-05-15
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-15
 * License: GPLv3
 */

use crate::consts::STACK_SIZE;
use crate::device::cpu;
use crate::thread::scheduler::scheduler;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::naked_asm;
use core::fmt::Display;
use core::sync::atomic::AtomicUsize;
use core::{fmt, ptr};

static THREAD_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn next_id() -> usize {
    THREAD_ID_COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}

/// Low-level routine for starting a thread.
#[unsafe(naked)]
unsafe extern "C" fn thread_start(stack_ptr: usize) {
    naked_asm!(
        "mov rsp, rdi", // Set the stack pointer to the prepared stack
        // We need to unlock the scheduler here to prevent deadlocks
        "call unlock_scheduler",
        "popfq", // Set rflags so that interrupts are disabled
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        // The address to the kickoff function is now at the top of the stack and we will call it
        "ret"
    )
}

/// Low-level routine for switching to the next thread.
/// * `current_stack_ptr` is a pointer to `stack_ptr` of the next coroutine (where the rsp is saved).
/// * `next_stack` is the value of `stack_ptr` of the next thread (the new rsp value).
/// * `next_stack_end` is the next stack's end address (new rsp0 value in TSS)
#[unsafe(naked)]
unsafe extern "C" fn thread_switch(
    current_stack_ptr: *mut usize,
    next_stack: usize,
    next_stack_end: usize,
) {
    naked_asm!(
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "pushfq",         // rflags
        "mov [rdi], rsp", // Save the current stack pointer
        // Before setting rsp: set rsp0 in the TSS
        // The function expects rsp0 in rdi
        "mov rdi, rdx", // Move next_stack_end into rdi
        "call tss_set_rsp0",
        "mov rsp, rsi", // Switch to the next stack
        // Unlock the scheduler here to prevent deadlocks
        "call unlock_scheduler",
        "popfq",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        // Depending on whether we ran this function before, we will either return to the kickoff
        // function (first time) or to the point where we switched away from this thread
        "ret"
    )
}

#[unsafe(naked)]
/// Called by `switch_to_usermode()`.
/// Switches to the prepared user stack and uses `iret` to return to ring 3 code.
unsafe extern "C" fn thread_user_start(stack_ptr: usize) {
    naked_asm!(
        "mov rsp, rdi", // Switch stack to the fake interrupt frame
        "pop rdi",
        "iretq" // Return to user mode (as if we were returning from an interrupt)
    )
}

/// Represents a thread in the system.
/// It contains the stack and the entry function.
/// Threads must be registered in the scheduler and are run automatically once the scheduler is started.
#[repr(C)]
pub struct Thread {
    id: usize,
    is_kernel_thread: bool,
    kernel_stack: Vec<u64>, // Memory for the kernel stack
    user_stack: Vec<u64>,   // Memory for the user stack
    stack_ptr: usize,       // Pointer on the stack to the saved context
    entry: fn(),
}

impl Thread {
    pub fn new_user_thread(entry: fn()) -> Box<Thread> {
        let mut thread = Self::new_kernel_thread(entry);
        thread.is_kernel_thread = false;
        thread
    }

    pub fn switch_to_usermode(&mut self) {
        // Fake interrupt frame on the kernel stack
        // Write it 1 KiB below the top to leave room for frames from previously called functions
        let offset = self.kernel_stack.len() - 1024 / 8;

        // Segment Selector (SS): GDT index = 4, RPL = 3
        self.kernel_stack[offset] = (4 << 3) | 3;

        // RSP -> Top of user stack
        let mut rsp = Self::get_top_of_stack(&self.user_stack) as u64;
        // Ensure that rsp % 16 == 8 when kickoff_user_thread is entered
        // iretq does not push a return address, so we ensure it here
        rsp &= !0xf; // clear the lowest 4 bits
        rsp -= 8;
        self.kernel_stack[offset - 1] = rsp;

        let mut rflags = 0u64;
        rflags |= 1 << 9; // Bit 9: IF = Interrupt Enable Flag = 1
        rflags |= 1 << 1; // Bit 1: reserved, must stay 1
        self.kernel_stack[offset - 2] = rflags;

        // Code Segment (CS): GDT index = 3, RPL = 3
        self.kernel_stack[offset - 3] = (3 << 3) | 3;

        // RIP -> where iretq jumps to
        self.kernel_stack[offset - 4] = (Thread::kickoff_user_thread as *const ()) as u64;

        // Argument to kickoff_user_thread()
        self.kernel_stack[offset - 5] = ptr::from_mut(self) as u64;

        let stack_ptr = ptr::from_ref(&self.kernel_stack[offset - 5]) as usize;

        unsafe { thread_user_start(stack_ptr) };
    }

    /// Called indirectly by using the prepared stack in 'thread_start' and 'thread_switch'.
    pub fn new_kernel_thread(entry: fn()) -> Box<Thread> {
        // Allocate memory for the stack and initialize it to zero
        let mut kernel_stack = Vec::<u64>::with_capacity(STACK_SIZE / 8);
        for _ in 0..kernel_stack.capacity() {
            kernel_stack.push(0);
        }

        let mut user_stack = Vec::<u64>::with_capacity(STACK_SIZE / 8);
        for _ in 0..user_stack.capacity() {
            user_stack.push(0);
        }

        // Set the stack pointer to the top of the stack
        let stack_ptr = ptr::from_ref(&kernel_stack[kernel_stack.capacity() - 1]) as usize;

        // Create a new thread object
        let mut thread = Box::new(Thread {
            id: next_id(),
            is_kernel_thread: true,
            kernel_stack,
            user_stack,
            stack_ptr,
            entry,
        });

        // Prepare the stack for the thread so it can be started via `thread_start()`
        thread.prepare_kernel_stack();
        thread
    }

    /// Start the thread.
    /// This function is only once by the scheduler.
    /// The scheduler does further thread switching via `switch()`.
    pub fn start(&mut self) {
        unsafe {
            thread_start(self.stack_ptr);
        }
    }

    /// Switch from the `current` thread to the `next` thread.
    /// This function is called by the scheduler to switch between threads.
    pub unsafe fn switch(current: *mut Thread, next: *mut Thread) {
        unsafe {
            let next_stack_end = Thread::get_top_of_stack(&(*next).kernel_stack) as usize;
            thread_switch(&mut (*current).stack_ptr, (*next).stack_ptr, next_stack_end);
        }
    }

    /// Get the ID of the thread.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Prepare the stack of a newly created thread in a way that it can be used
    /// to return to the 'kickoff' function with the thread itself as parameter.
    /// The prepared stack is used in 'thread_start' to start the first thread.
    /// Other threads are started by 'thread_switch' with the prepared stack.
    fn prepare_kernel_stack(&mut self) {
        let kickoff = (Thread::kickoff_kernel_thread as *const ()) as u64;
        let thread = ptr::from_mut(self) as u64;
        let length = self.kernel_stack.len();

        self.kernel_stack[length - 1] = kickoff; // Address of 'kickoff'
        self.kernel_stack[length - 2] = 0; // r8
        self.kernel_stack[length - 3] = 0; // r9
        self.kernel_stack[length - 4] = 0; // r10
        self.kernel_stack[length - 5] = 0; // r11
        self.kernel_stack[length - 6] = 0; // r12
        self.kernel_stack[length - 7] = 0; // r13
        self.kernel_stack[length - 8] = 0; // r14
        self.kernel_stack[length - 9] = 0; // r15
        self.kernel_stack[length - 10] = 0; // rax
        self.kernel_stack[length - 11] = 0; // rbx
        self.kernel_stack[length - 12] = 0; // rcx
        self.kernel_stack[length - 13] = 0; // rdx
        self.kernel_stack[length - 14] = 0; // rsi
        self.kernel_stack[length - 15] = thread; // rdi -> First parameter for 'kickoff'
        self.kernel_stack[length - 16] = 0; // rbp
        self.kernel_stack[length - 17] = 0x2; // rflags (IE = 0); interrupts disabled

        self.stack_ptr = self.stack_ptr - (size_of::<u64>() * 16);
    }

    /// Called indirectly by using the prepared stack in 'thread_start' and 'thread_switch'.
    fn kickoff_kernel_thread(&mut self) {
        // Set rsp0 in TSS
        let rsp0 = Self::get_top_of_stack(&self.kernel_stack) as usize;
        unsafe { tss_set_rsp0(rsp0) }

        if self.is_kernel_thread {
            // Interrupts are disabled during thread start, so we need to re-enable them here
            cpu::enable_int();
            ((*self).entry)();
        } else {
            self.switch_to_usermode();
        }

        scheduler().exit();
    }

    fn kickoff_user_thread(&self) {
        ((*self).entry)();
        loop {} // User threads may currently not exit (iretq jumps (no call) so there is no return address on the stack)
    }

    /// Get a pointer to the top of the given stack.
    fn get_top_of_stack(stack: &Vec<u64>) -> *const u64 {
        unsafe { ptr::from_ref(&stack[stack.len() - 1]).offset(1) }
    }
}

unsafe extern "C" {
    fn tss_set_rsp0(rsp0: usize);
}

impl Display for Thread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "T{}", self.id)
    }
}
