/*
 * Contains a demo for heap allocations.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-14
 * License: GPLv3
 */

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use crate::allocator;
use crate::allocator::global::dump_free_list;
use crate::device::key::Scancode;
use crate::device::keyboard::KEYBOARD;
use crate::device::speaker;
use crate::device::speaker::SPEAKER;
use crate::device::terminal::terminal;

struct MyStruct {
    a: u8,
    b: u16,
    c: u32,
}

/// A simple heap demo, allocating and freeing memory on the heap.
/// The allocator state is dumped before and after each operation.
pub fn heap_demo() {
    let numbers = vec![1, 2, 3, 4, 5];
    let mut squared = vec![];

    dump_free_list();

    println!("Numbers:");
    for i in numbers.iter() {
        println!("{} ", i);

        squared.push(i * i);
    }

    dump_free_list();

    println!("Squared:");
    for i in squared.iter() {
        println!("{} ", i);
    }

    dump_free_list();

    let my_struct = Box::new(MyStruct { a: 1, b: 2, c: 3 });

    dump_free_list();

    println!("MyStruct:");
    println!("a: {}", my_struct.a);
    println!("b: {}", my_struct.b);
    println!("c: {}", my_struct.c);

    drop(my_struct);
    drop(squared);
    drop(numbers);

    dump_free_list();
}

/// A demo that plays songs via the PC speaker.
pub fn speaker_demo() {
    todo!("lesson2::speaker_demo() is not implemented yet.")
}