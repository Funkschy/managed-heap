use crate::block::Block;
use crate::block_vec::BlockVec;

use core::ptr::NonNull;
use std::alloc::{alloc, dealloc, Layout};
use std::mem;
use std::u16;

struct Heap {
    size: usize,
    data: *mut usize,
    heap_end: usize,
    layout: Layout,
    free_blocks: BlockVec,
    used_blocks: BlockVec,
}

impl Heap {
    const ALIGN: u16 = mem::align_of::<usize>() as u16;
    const H_SIZE: u16 = mem::size_of::<usize>() as u16;

    pub unsafe fn new(layout: Layout) -> Self {
        let size = layout.size();

        if size > u16::MAX as usize {
            panic!("Size too big (MAX: {})", u16::MAX);
        }

        let data = NonNull::new(alloc(layout))
            .unwrap()
            .cast::<usize>()
            .as_ptr();

        let heap_end = data.add(size) as usize;

        Heap {
            size,
            data,
            heap_end,
            layout,
            free_blocks: BlockVec::from_raw(data, size as u16),
            used_blocks: BlockVec::default(),
        }
    }
}

impl Heap {
    fn round_up(n: u16, m: u16) -> u16 {
        ((n + m - 1) / m) * m
    }

    fn is_free(&self, block: Block) -> bool {
        self.free_blocks.contains(block)
    }
}

impl Heap {
    fn alloc(&mut self, size: u16) -> Option<Block> {
        let total_size = Heap::round_up(size + Heap::H_SIZE, Heap::ALIGN);
        let mut block = self.free_blocks.get_block(total_size)?;

        if block.size() > (total_size + Heap::H_SIZE * 2) {
            unsafe {
                let (first, second) = block.split_after(total_size);
                block = first;
                self.free_blocks.add_block(second);
            }
        }

        self.used_blocks.add_block(block);
        Some(block)
    }

    fn free(&mut self, mut block: Block) {
        let mut size = block.size();

        let next_block = block.next_block(self.heap_end);
        if let Some(next) = next_block {
            if self.is_free(next) {
                self.free_blocks.remove(block);
                size += next.size();
            }
        }

        let pred_block = block.pred_block(self.data as usize);
        if let Some(mut pred) = pred_block {
            if self.is_free(pred) {
                pred.inc_size(size);
                size = pred.size();
            } else {
                block.set_size(size);
                self.free_blocks.add_block(block);
            }
        }

        let after_next = next_block.map(|next| next.next_block(self.heap_end));
        if let Some(Some(mut after)) = after_next {
            after.set_pred_size(size);
        }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.data as *mut u8, self.layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_alloc_returns_correct_size_when_not_aligned() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let address = heap.alloc(10).unwrap();
            let expected;

            #[cfg(target_pointer_width = "64")]
            {
                // (header size (8) + 10) rounded to next multiple of 8
                expected = 24;
            }

            #[cfg(target_pointer_width = "32")]
            {
                // (header size (4) + 10) rounded to next multiple of 4
                expected = 16;
            }

            assert_eq!(expected, address.size());
        }
    }

    #[test]
    fn test_alloc_returns_correct_size_when_aligned() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let address = heap.alloc(16).unwrap();
            let expected;

            #[cfg(target_pointer_width = "64")]
            {
                // (header size (8) + 16)
                expected = 24;
            }

            #[cfg(target_pointer_width = "32")]
            {
                // (header size (4) + 16)
                expected = 20;
            }

            assert_eq!(expected, address.size());
        }
    }

    #[test]
    fn test_alloc_zero_size_should_return_header_size() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let address = heap.alloc(0).unwrap();
            let expected = mem::size_of::<usize>() as u16;

            assert_eq!(expected, address.size());
        }
    }

    #[test]
    fn test_alloc_splits_heap_block() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            heap.alloc(10).unwrap();

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(1, heap.used_blocks.len());

            heap.alloc(29).unwrap();
            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(2, heap.used_blocks.len());

            heap.alloc(0).unwrap();
            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(3, heap.used_blocks.len());
        }
    }
}
