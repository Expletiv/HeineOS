/*
 * A Basic heap allocator which cannot deallocate memory.
 * It allocates memory by simply increasing a pointer and is only intended for learning and testing purposes.
 *
 * Author: Philipp Oppermann, https://os.phil-opp.com/allocator-designs/
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-13
 */

use alloc::alloc::{GlobalAlloc, Layout};
use crate::allocator::global::{align_up, Locked};

/// A simple bump allocator that allocates memory in a linear fashion.
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Create a new empty bump allocator.
    pub const fn new() -> BumpAllocator {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize the bump allocator.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }

    /// Dump free memory for debugging purposes.
    pub fn dump_free_list(&self) {
        log::info!("BumpAllocator free region: [{:#x} - {:#x}] ({} bytes free, {} allocations active)",
        self.next,
        self.heap_end,
        self.heap_end - self.next,
        self.allocations,
    );
    }

    /// Allocate memory of the given size and alignment.
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        // Align the pointer to the required alignment
        let aligned_ptr = align_up(self.next, align);

        self.next = aligned_ptr.wrapping_add(size);
        self.allocations += 1;

        aligned_ptr as *mut u8
    }

    /// Deallocate memory (not supported by bump allocator).
    pub unsafe fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {
        self.allocations -= 1;

        // If the allocator is empty, reset everything at once.
        if self.allocations == 0 {
            self.next = self.heap_start;
        }
    }
}

// Trait required by the Rust runtime for heap allocations
unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            self.lock().alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.lock().dealloc(ptr, layout);
        }
    }
}
