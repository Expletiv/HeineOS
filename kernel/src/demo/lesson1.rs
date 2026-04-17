/*
 * Contains demos for text output and keyboard input.
 *
 * Author: Michael Schoetter, Heinrich Heine University Duesseldorf
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-14
 * License: GPLv3
 */

use crate::device::terminal::terminal;

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
    todo!("lesson1::keyboard_demo() not implemented yet");
}