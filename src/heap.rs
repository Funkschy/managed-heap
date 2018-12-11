use crate::address::Address;
use crate::block::Block;
use crate::block_set::BlockSet;

use core::ptr::NonNull;
use std::alloc::{alloc, dealloc, Layout};
use std::mem;
use std::u16;

pub struct Heap {
    size: usize,
    data: *mut usize,
    heap_end: usize,
    layout: Layout,
    free_blocks: BlockSet,
    used_blocks: BlockSet,
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
            free_blocks: BlockSet::from_raw(data, size as u16),
            used_blocks: BlockSet::default(),
        }
    }
}

impl Heap {
    fn round_up(n: u16, m: u16) -> u16 {
        // division basically works as floor
        ((n + m - 1) / m) * m
    }

    fn is_free(&self, block: Block) -> bool {
        self.free_blocks.contains(block)
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Heap {
    pub fn alloc<'a>(&mut self, size: u16) -> Option<Address<'a>> {
        let block = self.alloc_block(size)?;
        self.used_blocks.add_block(block);
        Some(Address::from(block))
    }

    fn alloc_block(&mut self, size: u16) -> Option<Block> {
        let total_size = Heap::round_up(size + Heap::H_SIZE, Heap::ALIGN);
        let mut block = self.free_blocks.get_block(total_size)?;

        if block.size() > (total_size + Heap::H_SIZE * 2) {
            unsafe {
                let (first, second) = block.split_after(total_size);
                block = first;
                self.free_blocks.add_block(second);
            }
        }

        Some(block)
    }

    pub fn free<'a>(&mut self, address: Address<'a>) {
        let mut block: Block = address.into();
        self.used_blocks.remove_block(block);

        let mut size = block.size();

        let next_block = block.next_block(self.heap_end);
        let mut freed_next = false;

        if let Some(next) = next_block {
            if self.is_free(next) {
                self.free_blocks.remove_block(next);
                size += next.size();
                freed_next = true;
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
        } else {
            block.set_size(size);
            self.free_blocks.add_block(block);
        }

        if freed_next {
            let after_next = next_block.map(|next| next.next_block(self.heap_end));
            if let Some(Some(mut after)) = after_next {
                after.set_pred_size(size);
            }
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
    fn test_alloc_block_returns_correct_size_when_not_aligned() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let block = heap.alloc_block(10).unwrap();
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

            assert_eq!(expected, block.size());
        }
    }

    #[test]
    fn test_alloc_block_returns_correct_size_when_aligned() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let block = heap.alloc_block(16).unwrap();
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

            assert_eq!(expected, block.size());
        }
    }

    #[test]
    fn test_alloc_block_zero_size_should_return_header_size() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let block = heap.alloc_block(0).unwrap();
            let expected = mem::size_of::<usize>() as u16;

            assert_eq!(expected, block.size());
        }
    }

    #[test]
    fn test_alloc_block_splits_heap_block() {
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

    #[test]
    fn test_free_single_block() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());
            let address = heap.alloc(10).unwrap();

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(1, heap.used_blocks.len());

            heap.free(address);

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(0, heap.used_blocks.len());
        }
    }

    #[test]
    fn test_free_adjacent_blocks() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let first_address = heap.alloc(10).unwrap();
            let second_address = heap.alloc(50).unwrap();
            let third_address = heap.alloc(1024).unwrap();

            // [used] [used] [used] [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(3, heap.used_blocks.len());

            heap.free(first_address);

            // [free] [used] [used] [free]

            assert_eq!(2, heap.free_blocks.len());
            assert_eq!(2, heap.used_blocks.len());

            heap.free(third_address);

            // [free] [used] [free]

            assert_eq!(2, heap.free_blocks.len());
            assert_eq!(1, heap.used_blocks.len());

            heap.free(second_address);

            // [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(0, heap.used_blocks.len());
        }
    }

    #[test]
    fn test_alloc_block_and_free_entire_heap() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let size = 4096 - mem::size_of::<usize>();
            let address = heap.alloc(size as u16).unwrap();

            let block: Block = address.into();

            assert_eq!(None, block.pred_block(heap.data as usize));
            assert_eq!(None, block.next_block(heap.heap_end));
            assert_eq!(1, heap.used_blocks.len());
            assert_eq!(0, heap.free_blocks.len());

            heap.free(address);

            assert_eq!(0, heap.used_blocks.len());
            assert_eq!(1, heap.free_blocks.len());
        }
    }

    #[test]
    fn test_write_allocated_block() {
        unsafe {
            let layout = Layout::from_size_align(4096, mem::align_of::<usize>());
            let mut heap = Heap::new(layout.unwrap());

            let address = heap.alloc(1).unwrap();
            let mut block: Block = address.into();

            let expected = (2 * mem::size_of::<usize>()) as u16;
            assert_eq!(expected, block.size());

            block.write_at(0, 42);
            assert_eq!(42, *address);
        }
    }
}
