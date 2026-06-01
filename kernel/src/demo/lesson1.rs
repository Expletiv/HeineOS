/*
 * Contains demos for text output and keyboard input.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-14
 * License: GPLv3
 */
use crate::device::key::Scancode;
use crate::device::keyboard::keyboard_buffer;

/// A simple text demo, displaying formatted numbers.
pub fn text_demo() {
    println!("Text Demo:");
    println!("  | dec | hex | bin   |");
    println!("  |-----|-----|-------|");

    for i in 0..=16 {
        println!("  | {:3} | {:3x} | {:5b} |", i, i, i);
    }
}

/// A simple keyboard demo, displaying the events of key presses and releases.
pub fn keyboard_demo() {
    println!("Keyboard Demo:");
    println!("Press keys on your keyboard. Press 'Esc' to exit the demo.");

    let mut buffer = keyboard_buffer();

    loop {
        let event = buffer.poll_key_press();
        println!("{:?}", event);

        if let Some(Scancode::Escape) = event.scancode() {
            break;
        }
    }

    println!("Exiting keyboard demo.");
}