/*
 * A driver for the programmable interval timer (PIT).
 *
 * Author: Michael Schoettner, Heinrich Heine University Duesseldorf, 2023-06-15
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-15
 * License: GPLv3
 */

use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::device::cpu::IoPort;
use crate::device::{framebuffer, terminal};
use crate::device::framebuffer::Framebuffer;
use crate::device::pic::{Irq, PIC};
use crate::device::terminal::{framebuffer, terminal};
use crate::interrupt::dispatcher::{IntVectors, InterruptVector};
use crate::interrupt::isr::ISR;
use crate::library::once::Once;
use crate::library::spinlock::SpinlockGuard;
use crate::thread::scheduler::scheduler;

/// Get the current system time in milliseconds.
pub fn system_time() -> usize {
    SYSTEM_TIME.load(Ordering::Relaxed)
}

/// Wait for a specified number of milliseconds using the system time.
pub fn wait(ms: usize) {
    let start_time = system_time();
    while system_time() - start_time < ms {
        scheduler().yield_cpu();
    }
}

#[repr(u16)]
/// I/O port addresses for the PIT.
enum PitRegister {
    Control = 0x43,
    Data = 0x40
}

/// Frequency of the timer in Hz
const TIMER_FREQUENCY: usize = 1193182;

/// Nanoseconds that pass per timer tick
const NANOSECONDS_PER_TICK: usize = 1_000_000_000 / TIMER_FREQUENCY;

/// The interval at which the timer should generate interrupts (1 ms).
const TIMER_INTERRUPT_INTERVAL_MS: usize = 1;

/// Global timer instance
static TIMER: Once<Timer> = Once::new();

/// System time in milliseconds.
/// This variable is updated by the timer interrupt service routine.
static SYSTEM_TIME: AtomicUsize = AtomicUsize::new(0);

/// Characters used for the spinner animation.
static SPINNER_CHARS: &[char] = &['|', '/', '-', '\\'];

/// Register the timer interrupt handler.
pub fn plugin() {
    TIMER.init(|| {
        let mut timer = Timer::new();
        timer.set_interrupt_interval(TIMER_INTERRUPT_INTERVAL_MS);
        timer
    });
    PIC.lock().allow(Irq::Timer);
    IntVectors::register(InterruptVector::Pit, Box::new(TimerISR { interval_ms: TIMER_INTERRUPT_INTERVAL_MS }))
}

/// Represents the programmable interval timer.
struct Timer {
    control_port: IoPort,
    data_port0: IoPort
}

/// The timer interrupt service routine.
struct TimerISR {
    interval_ms: usize,
}

impl ISR for TimerISR {
    /// Handle the timer interrupt.
    /// This function updates the system time and triggers a context switch every 10 ms.
    fn trigger(&self) {
        // Increment system time
        let time = SYSTEM_TIME.fetch_add(self.interval_ms, Ordering::Relaxed);

        if time % 250 == 0 {
            // Every 250 ms, print a spinner character.
            let spinner_char = SPINNER_CHARS[(time / 250) % SPINNER_CHARS.len()];

            let framebuffer: Option<SpinlockGuard<Framebuffer>> = terminal::framebuffer().try_lock();
            if let Some(mut framebuffer) = framebuffer {
                framebuffer.draw_char(spinner_char, 256, 256, framebuffer::WHITE, framebuffer::BLACK);
            }
        }
    }
}

impl Timer {
    /// Create a new Timer instance.
    pub const fn new() -> Timer {
        Timer {
            control_port: IoPort::new(PitRegister::Control as u16),
            data_port0: IoPort::new(PitRegister::Data as u16)
        }
    }

    /// Set the timer interrupt interval in milliseconds.
    pub fn set_interrupt_interval(&mut self, interval_ms: usize) {
        if interval_ms == 0 {
            return;
        }

        let interval_ns = interval_ms as u64 * 1_000_000;
        let counter64 = interval_ns / NANOSECONDS_PER_TICK as u64;
        let mut counter = if counter64 == 0 { 1 } else if counter64 > 0xFFFF { 0xFFFF } else { counter64 as u16 };

        // Control word: channel 0, access lobyte/hibyte, mode 3 (square wave), binary
        const CONTROL_WORD: u8 = 0b00110110;

        unsafe {
            self.control_port.outb(CONTROL_WORD);
            // write counter low byte then high byte to channel 0 data port
            self.data_port0.outb((counter & 0xFF) as u8);
            self.data_port0.outb((counter >> 8) as u8);
        }
    }
}
