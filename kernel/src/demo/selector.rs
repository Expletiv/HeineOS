use crate::demo::{lesson1, lesson2, lesson4, lesson5, lesson6, lesson7};
use crate::device::terminal::terminal;
use crate::library::input;
use crate::library::input::read_char;
use crate::thread::scheduler::scheduler;
use crate::thread::thread::Thread;

pub fn select_demo() {
    let scheduler = scheduler();
    let demo_thread = Thread::new(demo_loop);
    scheduler.ready(demo_thread);
    scheduler.schedule();
}

fn demo_loop() {
    loop {
        terminal().lock().clear();

        println!("1. Text Demo");
        println!("2. Keyboard Demo");
        println!("3. Heap Demo");
        println!("4. Speaker Demo");
        println!("5. Coroutine Demo");
        println!("6. Thread Demo");
        println!("7. Peanut GB Demo");
        println!("8. Print PCI Devices");
        println!("9. RTL8139 Demo");

        let c = read_char();

        match c {
            '1'..='9' => {
                terminal().lock().clear();

                match c {
                    '1' => lesson1::text_demo(),
                    '2' => lesson1::keyboard_demo(),
                    '3' => lesson2::heap_demo(),
                    '4' => lesson2::speaker_demo(),
                    '5' => lesson4::coroutine_demo(),
                    '6' => lesson5::thread_demo(),
                    '7' => lesson6::peanut_gb::play("/roms/2048.gb"),
                    '8' => lesson7::print_pci_devices(),
                    '9' => lesson7::rtl8139_demo(),
                    _ => {}
                }

                println!("\nPress 'Enter' to return to the demo selector.");
                input::wait_for_return();
            }
            _ => {
                continue;
            }
        }
    }
}