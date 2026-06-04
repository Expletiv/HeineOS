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
    todo!("lesson4::thread_demo() is not implemented yet.");
}

/// The function executed by each thread in the thread demo.
/// It increments a counter and prints it to the terminal in an endless loop,
/// yielding the CPU to the next thread after each print.
fn thread_entry() {
    todo!("lesson4::thread_entry() is not implemented yet.");
}