#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};
use memory_addr::align_up;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    count: usize, // free when (= count 0)
    byte_ptr: usize,
    page_ptr: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            count: 0,
            byte_ptr: 0,
            page_ptr: 0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        *self = Self {
            start,
            end: start + size,
            count: 0,
            byte_ptr: start,
            page_ptr: start + size,
        };
    }
    fn add_memory(&mut self, _start: usize, _size: usize) -> allocator::AllocResult {
        panic!("Should not add memory in early allocator!");
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(
        &mut self,
        layout: core::alloc::Layout,
    ) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let align_ptr = align_up(self.byte_ptr, layout.align());
        let alloc_ptr = self.byte_ptr + layout.size();
        if alloc_ptr <= self.page_ptr {
            self.byte_ptr = align_ptr;
            self.count += 1;
            core::ptr::NonNull::new(alloc_ptr as *mut u8).ok_or(allocator::AllocError::NoMemory)
        } else {
            Err(allocator::AllocError::NoMemory)
        }
    }
    fn dealloc(&mut self, _pos: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        self.count -= 1;
        if self.count == 0 {
            self.byte_ptr = self.start;
        }
    }
    fn total_bytes(&self) -> usize {
        self.end - self.start
    }
    fn used_bytes(&self) -> usize {
        self.byte_ptr - self.start
    }
    fn available_bytes(&self) -> usize {
        self.page_ptr - self.byte_ptr
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;
    fn alloc_pages(
        &mut self,
        num_pages: usize,
        _align_pow2: usize,
    ) -> allocator::AllocResult<usize> {
        let alloc_ptr = self.byte_ptr - num_pages * Self::PAGE_SIZE;
        if alloc_ptr > self.byte_ptr {
            self.byte_ptr = alloc_ptr;
            Ok(alloc_ptr)
        } else {
            Err(allocator::AllocError::NoMemory)
        }
    }
    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        panic!("Should not dealloc pages in early allocator!");
    }
    fn total_pages(&self) -> usize {
        (self.end - self.start) / Self::PAGE_SIZE
    }
    fn used_pages(&self) -> usize {
        (self.end - self.page_ptr) / Self::PAGE_SIZE
    }
    fn available_pages(&self) -> usize {
        (self.page_ptr - self.byte_ptr) / Self::PAGE_SIZE
    }
}
