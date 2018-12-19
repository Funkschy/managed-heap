use crate::address::Address;
use crate::block::set::BlockSet;
use crate::block::Block;
use crate::types::*;

use core::ptr::NonNull;
use std::alloc::{alloc, dealloc, Layout};
use std::iter::Iterator;
use std::mem;

pub struct Heap {
    size: usize,
    data: *mut usize,
    heap_end: usize,
    layout: Layout,
    free_blocks: BlockSet,
    used_blocks: BlockSet,
}

impl Heap {
    const H_SIZE: HalfWord = mem::size_of::<usize>() as HalfWord;

    /// Expects the heap size in bytes.
    pub unsafe fn new(size: usize) -> Self {
        let align = mem::align_of::<usize>();
        let layout = Layout::from_size_align(size, align).unwrap();

        if size > HALF_WORD_MAX as usize {
            panic!("Size too big (MAX: {})", HALF_WORD_MAX);
        }

        let data = NonNull::new(alloc(layout))
            .unwrap()
            .cast::<usize>()
            .as_ptr();

        let size = size / Heap::H_SIZE as usize;
        let heap_end = data.add(size) as usize;

        Heap {
            size,
            data,
            heap_end,
            layout,
            free_blocks: BlockSet::from_raw(data, size as HalfWord),
            used_blocks: BlockSet::default(),
        }
    }
}

impl Heap {
    fn is_free(&self, block: Block) -> bool {
        self.free_blocks.contains(block)
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn num_used_blocks(&self) -> usize {
        self.used_blocks.len()
    }

    pub fn num_free_blocks(&self) -> usize {
        self.free_blocks.len()
    }
}

impl Heap {
    /// Takes the blocksize as a number of usize values.
    /// The size in bytes of the block is therefore size * mem::size_of::<usize>()
    /// (technically + one more usize to store information about the block)
    pub fn alloc(&mut self, size: HalfWord) -> Option<Address> {
        let block = self.alloc_block(size)?;
        self.used_blocks.add_block(block);
        Some(Address::from(block))
    }

    fn alloc_block(&mut self, size: HalfWord) -> Option<Block> {
        let total_size = size + 1;
        let mut block = self.free_blocks.get_block(total_size)?;

        if block.size() > (total_size + 2) {
            unsafe {
                let (first, second) = block.split_after(total_size);
                block = first;
                self.free_blocks.add_block(second);
            }
        }

        Some(block)
    }

    pub fn free(&mut self, address: Address) {
        // TODO clean up
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
        } else if let Some(mut next) = next_block {
            if !self.is_free(next) {
                next.set_pred_size(size);
            }
        }
    }
}

impl Heap {
    pub fn used<'a>(&'a self) -> Box<Iterator<Item = &Block> + 'a> {
        self.used_blocks.iter()
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

    #[test]
    fn test_alloc_block_returns_correct_size_when_not_aligned() {
        unsafe {
            let mut heap = Heap::new(4096);

            let block = heap.alloc_block(10).unwrap();
            let expected = 11;

            assert_eq!(expected, block.size());
        }
    }

    #[test]
    fn test_alloc_block_returns_correct_size_when_aligned() {
        unsafe {
            let mut heap = Heap::new(4096);

            let block = heap.alloc_block(16).unwrap();
            let expected = 17;

            assert_eq!(expected, block.size());
        }
    }

    #[test]
    fn test_alloc_block_zero_size_should_return_header_size() {
        unsafe {
            let mut heap = Heap::new(4096);

            let block = heap.alloc_block(0).unwrap();
            let expected = 1;

            assert_eq!(expected, block.size());
        }
    }

    #[test]
    fn test_alloc_block_splits_heap_block() {
        unsafe {
            let mut heap = Heap::new(4096);

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
            let mut heap = Heap::new(4096);
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
            let mut heap = Heap::new(4096);

            let first_address = heap.alloc(10).unwrap();
            let second_address = heap.alloc(50).unwrap();
            let third_address = heap.alloc(100).unwrap();

            // [used] [used] [used] [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(3, heap.used_blocks.len());

            let first_block: Block = first_address.into();
            let second_block: Block = second_address.into();
            let third_block: Block = third_address.into();

            assert_eq!(None, first_block.pred_block(heap.data as usize));
            assert_eq!(Some(second_block), first_block.next_block(heap.heap_end));
            assert_eq!(false, heap.is_free(first_block));

            assert_eq!(
                Some(first_block),
                second_block.pred_block(heap.data as usize)
            );
            assert_eq!(Some(third_block), second_block.next_block(heap.heap_end));
            assert_eq!(false, heap.is_free(second_block));

            assert_eq!(
                Some(second_block),
                third_block.pred_block(heap.data as usize)
            );
            assert!(third_block.next_block(heap.heap_end).is_some());
            assert!(heap.is_free(third_block.next_block(heap.heap_end).unwrap()));
            assert_eq!(false, heap.is_free(third_block));

            heap.free(Address::from(first_block));

            // [free] [used] [used] [free]

            assert_eq!(2, heap.free_blocks.len());
            assert_eq!(2, heap.used_blocks.len());

            heap.free(Address::from(third_block));

            // [free] [used] [free]

            assert_eq!(2, heap.free_blocks.len());
            assert_eq!(1, heap.used_blocks.len());

            heap.free(Address::from(second_block));

            // [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(0, heap.used_blocks.len());

            let size = (4096 - mem::size_of::<usize>()) / mem::size_of::<usize>();
            let entire = heap.alloc(size as HalfWord).unwrap();

            let entire_block: Block = entire.into();

            // [used]

            let size = (4096 - mem::size_of::<usize>()) / mem::size_of::<usize>();

            assert_eq!(size + 1, entire_block.size() as usize);
            assert_eq!(None, entire_block.pred_block(heap.data as usize));
            assert_eq!(None, entire_block.next_block(heap.heap_end));
            assert_eq!(0, heap.free_blocks.len());
            assert_eq!(1, heap.used_blocks.len());

            heap.free(Address::from(entire_block));

            // [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(0, heap.used_blocks.len());
        }
    }

    #[test]
    fn test_free_adjacent_blocks_list() {
        unsafe {
            let mut heap = Heap::new(4096);

            let first_address = heap.alloc(10).unwrap();
            let second_address = heap.alloc(50).unwrap();
            let third_address = heap.alloc(100).unwrap();

            // [used] [used] [used] [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(3, heap.used_blocks.len());

            heap.free(first_address);

            // [free] [used] [used] [free]

            assert_eq!(2, heap.free_blocks.len());
            assert_eq!(2, heap.used_blocks.len());

            heap.free(second_address);

            // [free] [used] [free]

            assert_eq!(2, heap.free_blocks.len());
            assert_eq!(1, heap.used_blocks.len());

            let block: Block = third_address.into();
            assert!(heap.is_free(block.pred_block(heap.data as usize).unwrap()));

            heap.free(Address::from(block));

            // [free]

            assert_eq!(1, heap.free_blocks.len());
            assert_eq!(0, heap.used_blocks.len());
        }
    }

    #[test]
    fn test_alloc_block_and_free_entire_heap() {
        unsafe {
            let mut heap = Heap::new(4096);

            let size = (4096 - mem::size_of::<usize>()) / mem::size_of::<usize>();
            let address = heap.alloc(size as HalfWord).unwrap();

            let block: Block = address.into();

            assert_eq!(1, heap.used_blocks.len());
            assert_eq!(0, heap.free_blocks.len());
            assert_eq!(None, block.pred_block(heap.data as usize));
            assert_eq!(None, block.next_block(heap.heap_end));
            assert_eq!(size + 1, block.size() as usize);

            heap.free(Address::from(block));

            assert_eq!(0, heap.used_blocks.len());
            assert_eq!(1, heap.free_blocks.len());
        }
    }

    #[test]
    fn test_write_allocated_block() {
        unsafe {
            let mut heap = Heap::new(4096);

            let address = heap.alloc(1).unwrap();
            let mut block: Block = address.into();

            let expected = 2;
            assert_eq!(expected, block.size());

            block.write_at(0, 42);
            assert_eq!(42, *Address::from(block));

            let next = block.next_block(heap.heap_end).unwrap();
            let n_size = 4096 / Heap::H_SIZE - 2;

            assert_eq!(n_size, next.size());
            assert_eq!(2, next.pred_size());
        }
    }

    #[test]
    fn test_alloc_too_big_returns_none() {
        unsafe {
            let mut heap = Heap::new(128);
            let size = 128 / mem::size_of::<usize>() as HalfWord - 1;

            heap.alloc(size).unwrap();
            assert_eq!(1, heap.used_blocks.len());
            assert_eq!(0, heap.free_blocks.len());

            let address = heap.alloc(0);
            assert_eq!(None, address);
        }
    }
}
