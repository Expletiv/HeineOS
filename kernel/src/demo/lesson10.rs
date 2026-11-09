/*
 * Contains a demo for physical memory allocations.
 *
 * Author: Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-03-06
 * License: GPLv3
 */

use crate::consts::{PAGE_SIZE, kernel_end};
use crate::paging::frames::{FRAME_ALLOCATOR, PhysAddr};

/// Test the page frame allocator by performing a few allocations and frees.
/// The free list is dumped after each operation and checked with `assert!()`.
pub fn page_frame_demo() {
    let (initial_blocks, initial_frames) = stats();

    println!("--- 1. INITIAL FREE LIST ---");
    dump_free_list();

    // Allocations are cut off the end of the first block that is large enough,
    // so c lies directly below b, and b directly below a.
    let a = alloc(1);
    let b = alloc(2);
    let c = alloc(3);
    println!("\na: {:?} (1 frame)\nb: {:?} (2 frames)\nc: {:?} (3 frames)", a, b, c);

    // The allocator must never hand out frames of the kernel image
    for addr in [a, b, c] {
        assert!(addr.raw() >= kernel_end() as u64, "Frame {:?} overlaps the kernel", addr);
        assert!(addr.raw().is_multiple_of(PAGE_SIZE as u64));
    }
    assert_eq!(stats().1, initial_frames - 6);

    println!("\n--- 2. AFTER ALLOCATING a, b, c ---");
    dump_free_list();

    // Write a pattern into frame a and read it back
    unsafe {
        a.as_mut_ptr::<u8>().write_bytes(0xAB, PAGE_SIZE);
        for i in 0..PAGE_SIZE {
            assert_eq!(*a.as_ptr::<u8>().add(i), 0xAB);
        }
    }
    println!("\nWrote and read back 0xAB in frame a");

    // b lies between c and a, which are both still allocated: a new block appears
    free(b, 2);
    assert_eq!(stats(), (initial_blocks + 1, initial_frames - 4));
    println!("\n--- 3. AFTER FREEING b (new block) ---");
    dump_free_list();

    // a lies directly behind b: merged with b
    free(a, 1);
    assert_eq!(stats(), (initial_blocks + 1, initial_frames - 3));
    println!("\n--- 4. AFTER FREEING a (merged with b) ---");
    dump_free_list();

    // c closes the gap: merged with the block before it and with b + a
    free(c, 3);
    assert_eq!(stats(), (initial_blocks, initial_frames));
    println!("\n--- 5. AFTER FREEING c (merged on both sides, back to the start) ---");
    dump_free_list();

    // A request larger than all free memory must fail
    let too_large = unsafe { FRAME_ALLOCATOR.lock().alloc_block(initial_frames + 1) };
    assert!(too_large.is_none());
    println!("\nAllocating {} frames failed as expected", initial_frames + 1);

    // Uncomment to check the double free detection (panics and halts the kernel):
    // let d = alloc(1);
    // free(d, 1);
    // free(d, 1);

    println!("\nAll page frame tests passed");
}

fn alloc(num_frames: usize) -> PhysAddr {
    unsafe { FRAME_ALLOCATOR.lock().alloc_block(num_frames) }.expect("Out of physical memory")
}

fn free(addr: PhysAddr, num_frames: usize) {
    unsafe { FRAME_ALLOCATOR.lock().free_block(addr, num_frames) }
}

fn stats() -> (usize, usize) {
    FRAME_ALLOCATOR.lock().free_list_stats()
}

fn dump_free_list() {
    FRAME_ALLOCATOR.lock().dump_free_list();
}
