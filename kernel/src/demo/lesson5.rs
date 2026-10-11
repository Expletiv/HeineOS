use crate::device::terminal::terminal;
use crate::thread::scheduler::scheduler;
use crate::thread::thread::Thread;

/// A demo function showcasing threads.
/// It starts three threads, each incrementing a counter and printing it to the terminal in an endless loop.
pub fn thread_demo() {
    println!("Thread Demo:");

    let a = Thread::new(thread_entry);
    let b = Thread::new(thread_entry);
    let c = Thread::new(thread_entry);

    let scheduler = scheduler();
    scheduler.ready(a);
    scheduler.ready(b);
    scheduler.ready(c);
}

/// The function executed by each thread in the thread demo.
/// It increments a counter and prints it to the terminal.
fn thread_entry() {
    let mut counter = 1;

    loop {
        let mut terminal = terminal().lock();
        let tid = scheduler().get_active_tid();

        match (tid % 3, counter) {
            (0, 501) => { return; }
            (1, 1001) => { return; }
            (2, 1501) => { return; }
            _ => {}
        }

        terminal.set_pos(8, 8 + (tid % 3) * 2);

        print_terminal!(&mut terminal, "Thread [{}]: {}", tid, counter);
        drop(terminal);

        if counter % 10 == 0 {
            scheduler().yield_cpu();
        }

        counter += 1;
    }
}
