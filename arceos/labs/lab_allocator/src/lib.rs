//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

// use allocator::{AllocResult, BaseAllocator, ByteAllocator};
use allocator::{AllocResult, BaseAllocator, BitmapPageAllocator, ByteAllocator, PageAllocator};
use axlog::ax_println;
use core::alloc::Layout;
use core::ptr::NonNull;
use kspin::SpinNoIrq;
use memory_addr::{align_down, align_up};

// pub type DefaultByteAllocator = allocator::TlsfByteAllocator;
const PAGE_SIZE: usize = 0x1000;
const MIN_HEAP_SIZE: usize = 0x8000;

/*
 * 应用程序将会按照如下模式分配和释放内存
 * indicator = 0: 分配 [;32+1], [;64+1], [;128+1], [;256+1]
 *                释放 [;32+1],          [;128+1]
 * indicator = 1: 分配 [;32+2], [;64+2], [;128+2], [;256+2]
 *                释放 [;32+2],          [;128+2]
 * indicator = 2: 分配 [;32+3], [;64+3], [;128+3], [;256+3]
 *                释放 [;32+3],          [;128+3]
 * */

const MAX_BLOCKS: usize = 1 << 20;
// Time Exchanges Space Allocator
pub struct LabByteAllocator {
    // [l, r, valid];
    // valid = 0 for free, 1 for taken, 2 for invalid
    // [start, end, 0] [0, 0, 2] [0, 0, 2] ...
    //   alloc: [start, r1, 1] [r1 + 1, end, 0], [0, 0, 2] ...
    // dealloc: [start,
    pool: [(usize, usize, u8); MAX_BLOCKS],
    start: usize,
    size: usize,
    end: usize,
}

impl LabByteAllocator {
    pub fn show(&self) {
        ax_println!(
            "({} KiB / {} KiB)",
            self.used_bytes() / 1024,
            self.total_bytes() / 1024,
        );
    }
    fn find<F>(&self, check: F) -> usize where F: FnMut(&&(usize, usize, u8)) -> bool {
        self.pool.iter().filter(check).map(|t| LabByteAllocator::get_block_size(t)).sum()
    }
    pub const fn new() -> Self {
        // ax_println!("LabByteAllocator initialized! ");
        Self {
            pool: [(usize::MAX, usize::MAX, 2); MAX_BLOCKS],
            start: 0,
            size: 0,
            end: 0,
        }
    }
    fn get_block_size(block: &(usize, usize, u8)) -> usize {
        block.1 - block.0
    }
}
impl BaseAllocator for LabByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.size = size;
        self.end = start + size;
        self.pool[0] = (start, self.end, 0);
        ax_println!("Byte Allocator Init: Total {} Bytes", self.size);
    }
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        *(self.pool.iter_mut().find(|triple| triple.2 == 2).unwrap()) = (start, start + size, 0);
        Ok(())
    }
}
impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        assert!(layout.size() > 0);
        let mut splited_l = 0;
        let mut splited_r = 0;
        let mut ret = 0;

        let triple = self
            .pool
            .iter_mut()
            .filter(|triple| {
                triple.2 == 0 && LabByteAllocator::get_block_size(triple) >= layout.size()
            })
            .min_by_key(|triple| LabByteAllocator::get_block_size(triple));
        if let Some(triple) = triple {
            splited_l = triple.0 + layout.size();
            splited_r = triple.1;
            *triple = (triple.0, triple.0 + layout.size(), 1);
            ret = triple.0;
        } else {
            return Err(allocator::AllocError::NoMemory);
        }

        *(self.pool.iter_mut().find(|triple| triple.2 == 2).unwrap()) = (splited_l, splited_r, 0);
        unsafe { Ok(NonNull::new_unchecked(ret as *mut u8)) }
    }
    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        if let Some(triple) = self
            .pool
            .iter_mut()
            .find(|triple| triple.0 == pos.as_ptr() as usize)
        {
            assert_eq!(LabByteAllocator::get_block_size(triple), layout.size());
            *triple = (triple.0, triple.1, 0);
        }
        self.pool.sort_unstable_by_key(|triple| triple.0);
        for i in 0..self.pool.len()-1 {
            let (xl, xr, xv) = self.pool[i];
            let (yl, yr, yv) = self.pool[i + 1];
            if xr == yl && xv == 0 && yv == 0 {
                self.pool[i].1 = yr;
                self.pool[i + 1] = (usize::MAX, usize::MAX, 2);
            }
        }
    }
    fn total_bytes(&self) -> usize {
        self.find(|t| t.2 !=2)
    }
    fn available_bytes(&self) -> usize {
        self.find(|t| t.2 == 0)
    }
    fn used_bytes(&self) -> usize {
        self.find(|t| t.2 == 1)
    }
}
