/*
 * A heap allocator that uses a linked list to manage free memory blocks.
 * It allows for dynamic memory allocation and deallocation.
 *
 * Author: Philipp Oppermann, https://os.phil-opp.com/allocator-designs/
 *         Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-13
 */

use alloc::alloc::{GlobalAlloc, Layout};
use log::info;
use crate::allocator::global::{align_up, Locked};

/// Header of a free block in the list allocator.
struct ListNode {
    /// Size of the memory block
    size: usize,

    /// &'static mut type semantically describes an owned object behind a pointer.
    /// Basically, it’s a Box without a destructor that frees the object at the end of the scope.
    /// Its lifetime is static, meaning it will live for the entire duration of the program.
    /// Of course, this is not true in reality, as we might delete the list node at some point.
    /// But the compiler does not know this.
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    /// Create a new ListNode with the given size and no next node.
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    /// Get the start address of the memory block.
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    /// Get the end address of the memory block.
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// A linked list allocator that uses a free list to manage memory.
pub struct LinkedListAllocator {
    head: ListNode,
    heap_start: usize,
    heap_end: usize,
}

impl LinkedListAllocator {
    /// Create a new empty linked list allocator.
    pub const fn new() -> LinkedListAllocator {
        LinkedListAllocator {
            head: ListNode::new(0),
            heap_start: 0,
            heap_end: 0,
        }
    }

    /// Initialize the allocator with the heap bounds given in the constructor.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_block(heap_start, heap_size);
        }

        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;

        info!("List allocator initialized: 0x{:x} - 0x{:x} (Size: {} bytes)", heap_start, heap_start + heap_size, heap_size);
    }

    /// Adds the given free memory block 'addr' to the front of the free list.
    unsafe fn add_free_block(&mut self, addr: usize, size: usize) {
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;

        unsafe {
            node_ptr.write(node);
            self.head.next = Some(&mut *node_ptr)
        }
    }

    /// Search a free block with the given size and alignment and remove it from the list.
    fn find_free_block(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut block) = current.next {
            if let Ok(alloc_start) = Self::check_block_for_alloc(block, size, align) {
                let next = block.next.take();

                // block is large enough, remove it from the list
                let ret = Some((current.next.take().unwrap(), alloc_start));

                // link the previous block to the next block
                current.next = next;

                return ret;
            } else {
                // block too small, try next block
                current = current.next.as_mut().unwrap();
            }
        }

        // no block found
        None
    }

    /// Check if the given block is large enough for an allocation with `size` and `align`.
    fn check_block_for_alloc(block: &ListNode, size: usize, align: usize) -> Result<usize,()> {
        let alloc_start = align_up(block.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > block.end_addr() {
            return Err(());
        }

        // Don't allow the block if the excess is not enough to store a ListNode
        let excess_size = block.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < size_of::<ListNode>() {
            return Err(());
        }

        Ok(alloc_start)
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// block is also capable of storing a `ListNode`.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(size_of::<ListNode>());

        (size, layout.align())
    }

    /// Dump the free list for debugging purposes.
    pub fn dump_free_list(&self) {
        log::info!("Free memory blocks:");
        let mut current = &self.head;

        while let Some(ref block) = current.next {
            log::info!("  {:#x} - {:#x}", block.start_addr(), block.end_addr());

            current = block;
        }
    }

    /// Allocate memory of the given size and alignment.
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::size_align(layout);

        let Some((node, alloc_start)) = self.find_free_block(size, align) else {
            return core::ptr::null_mut();
        };

        // split the block if it has excess
        let alloc_end = alloc_start.checked_add(size).expect("overflow");

        let excess_size = node.end_addr() - alloc_end;
        if excess_size > 0 {
            // check_block_for_alloc() ensures that the excess is large enough to store a ListNode
            unsafe {
                self.add_free_block(alloc_end, excess_size);
            }
        }

        // return the start address of the aligned memory block (might leak memory at the start of the block)
        alloc_start as *mut u8
    }

    /// Free the memory block at the given pointer with the given layout.
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let (size, _) = Self::size_align(layout);

        unsafe {
            self.add_free_block(ptr as usize, size)
        }
    }
}

// Trait required by the Rust runtime for heap allocations
unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
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