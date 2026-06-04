/*
 * Contains demos for coroutines and threads.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-15
 * License: GPLv3
 */
use log::info;
use crate::coroutine::coroutine::Coroutine;
use crate::device::terminal::terminal;
use crate::thread::scheduler::scheduler;
use crate::thread::thread::Thread;

/// A demo function showcasing coroutines.
/// It starts three coroutines, each incrementing a counter and printing it to the terminal in an endless loop.
/// The coroutines switch to the next coroutine after each print.
pub fn coroutine_demo() {
    println!("Coroutine Demo:");

    let mut a = Coroutine::new(coroutine_loop);
    let mut b = Coroutine::new(coroutine_loop);
    let mut c = Coroutine::new(coroutine_loop);

    a.set_next(&mut b);
    b.set_next(&mut c);
    c.set_next(&mut a);

    a.start();
}

/// The function executed by each coroutine in the coroutine demo.
/// It increments a counter and prints it to the terminal in an endless loop,
/// switching to the next coroutine after each print.
fn coroutine_loop(coroutine: &mut Coroutine) {
    let mut counter = 1;

    loop {
        let mut terminal = terminal().lock();
        terminal.set_pos(8, 8 + coroutine.id() * 2);

        print_terminal!(&mut terminal, "Coroutine [{}]: {}", coroutine.id(), counter);
        drop(terminal);

        counter += 1;
        coroutine.switch();
    }
}

/// A demo function showcasing threads.
/// It starts three threads, each incrementing a counter and printing it to the terminal in an endless loop.
/// The threads yield the CPU to the next thread after each print.
/// The first thread also kills the other two threads after a certain number of iterations and finally exits itself, ending the demo.
pub fn thread_demo() {
    println!("Thread Demo:");

    let mut a = Thread::new(thread_entry);
    let mut b = Thread::new(thread_entry);
    let mut c = Thread::new(thread_entry);

    let scheduler = scheduler();
    scheduler.ready(a);
    scheduler.ready(b);
    scheduler.ready(c);

    scheduler.schedule();
}

/// The function executed by each thread in the thread demo.
/// It increments a counter and prints it to the terminal in an endless loop,
/// yielding the CPU to the next thread after each print.
fn thread_entry() {
    let mut counter = 1;

    loop {
        let mut terminal = terminal().lock();
        let scheduler = scheduler();
        let tid = scheduler.get_active_tid();

        if tid == 0 {
            if counter == 1501 {
                scheduler.kill(2);
            }

            if counter == 3001 {
                scheduler.kill(1);
            }

            if counter == 5001 {
                terminal.set_pos(8, 5);
                print_terminal!(&mut terminal, "Thread [{}] killed the other threads and exits itself.", tid);

                scheduler.exit();
            }
        }

        terminal.set_pos(8, 8 + tid * 2);

        print_terminal!(&mut terminal, "Thread [{}]: {}", tid, counter);
        drop(terminal);

        counter += 1;
        scheduler.yield_cpu();
    }
}