/*
 * Contains a demo for testing system calls.
 *
 * Author: Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-05-28
 * License: GPLv3
 */

use alloc::format;
use alloc::string::String;

use crate::syscall::user_api::{
    usr_get_key_event, usr_get_system_time, usr_hello_world, usr_print, usr_thread_get_id,
    usr_thread_wait,
};
use crate::thread::scheduler::scheduler;
use crate::thread::thread::Thread;
use core::sync::atomic::{AtomicBool, Ordering};

static DEMO_RUNNING: AtomicBool = AtomicBool::new(false);

/// Start the syscall demo.
/// It reads keyboard input and prints to the terminal from a user thread.
pub fn syscall_demo() {
    let thread = Thread::new_user_thread(syscall_test_thread);
    let scheduler = scheduler();

    DEMO_RUNNING.store(true, Ordering::Relaxed);
    scheduler.ready(thread);

    while DEMO_RUNNING.load(Ordering::Relaxed) {
        // wait until the demo has finished
        scheduler.yield_cpu();
    }
}

/// Thread function for the syscall demo.
/// It reads a line from the keyboard, and once the user presses 'Enter',
/// the given line and the current system time are printed to the terminal.
fn syscall_test_thread() {
    usr_hello_world();

    usr_print(
        "System Call Demo: Type something and press enter to see the output.\nType 'exit' to exit the thread.\n\n",
    );

    let tid = usr_thread_get_id();
    let tid_print = format!("Thread ID: {}\n\n", tid);
    usr_print(&tid_print);

    loop {
        let mut input = String::new();

        loop {
            let c = read_char();

            match c {
                '\r' => break,
                _ => input.push(c),
            };
        }

        if input == "exit" {
            DEMO_RUNNING.store(false, Ordering::Relaxed);

            return;
        }

        let you_typed = format!("You typed: '{}'\n", input);
        usr_print(&you_typed);

        let sytem_time = usr_get_system_time();
        let system_time_print = format!("System time: {}s\n\n", sytem_time / 1000);
        usr_print(&system_time_print);
    }
}

fn read_char() -> char {
    loop {
        let key = usr_get_key_event();
        if let Some(key) = key
            && key.pressed()
            && key.ascii().is_some()
        {
            return key.ascii().unwrap();
        }

        usr_thread_wait(100);
    }
}
