use self::header::BlockHeader;
use super::types::{HalfWord, WORD_SIZE};

use std::cmp::Ordering;
use std::fmt;
use std::ptr::NonNull;

pub mod header;
pub mod set;

#[derive(Copy, Clone)]
pub struct Block(NonNull<BlockHeader>);

impl Block {
    /// Takes a ptr to allocated memory of the specified size in usizes
    pub fn new(ptr: *mut usize, size: HalfWord, pred_size: HalfWord) -> Self {
        let header = BlockHeader::new(pred_size, size);
        unsafe {
            *ptr = header.into();

            Block(
                NonNull::new(ptr as *mut BlockHeader)
                    .expect("Cannot construct Block from NULL pointer"),
            )
        }
    }
}

impl Block {
    /// Writes value to memory after offset * size_of::<usize>() bytes.
    pub fn write_at(&mut self, offset: HalfWord, value: usize) {
        assert!(
            (offset as usize * WORD_SIZE) < self.size() as usize,
            "Offset is out of bounds"
        );

        unsafe {
            // add one to offset, to skip header
            *(self.0.as_ptr() as *mut usize).add(1 + offset as usize) = value;
        }
    }

    pub fn inc_size(&mut self, value: HalfWord) {
        unsafe {
            self.0.as_mut().inc_size(value);
        }
    }

    pub fn set_size(&mut self, value: HalfWord) {
        unsafe {
            self.0.as_mut().set_size(value);
        }
    }

    pub fn set_pred_size(&mut self, value: HalfWord) {
        unsafe {
            self.0.as_mut().set_pred_size(value);
        }
    }
}

impl Block {
    pub fn size(self) -> HalfWord {
        unsafe { self.0.as_ref().block_size() }
    }

    pub fn pred_size(self) -> HalfWord {
        unsafe { self.0.as_ref().pred_block_size() }
    }

    pub fn next_block(self, heap_end: usize) -> Option<Block> {
        let next_ptr = unsafe { self.0.as_ptr().add(self.size() as usize) };

        if next_ptr as usize >= heap_end {
            return None;
        }

        NonNull::new(next_ptr).map(Block)
    }

    pub fn pred_block(self, heap_start: usize) -> Option<Block> {
        let pred_size = self.pred_size();

        if pred_size == 0 {
            return None;
        }

        let offset = -(pred_size as isize);
        let pred_ptr = unsafe { self.0.as_ptr().offset(offset) };

        if (pred_ptr as usize) < heap_start {
            return None;
        }

        NonNull::new(pred_ptr).map(Block)
    }

    /// Splits the block by inserting a new header at self + size
    pub unsafe fn split_after(self, size: HalfWord) -> (Block, Block) {
        let current_size = self.size();
        assert!(current_size > size, "size too big");

        let pred_size = self.pred_size();

        let second_size = current_size - size;
        let ptr = self.0.as_ptr() as *mut usize;

        let second_ptr = ptr.add(size as usize);
        *second_ptr = BlockHeader::new(size, second_size).into();
        let second = Block(NonNull::new_unchecked(second_ptr as *mut BlockHeader));

        *ptr = BlockHeader::new(pred_size, size).into();

        (self, second)
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Block (pred: {}, size: {})",
            self.pred_size(),
            self.size()
        )
    }
}

impl Into<NonNull<BlockHeader>> for Block {
    fn into(self) -> NonNull<BlockHeader> {
        self.0
    }
}

impl From<*mut BlockHeader> for Block {
    fn from(value: *mut BlockHeader) -> Self {
        Block(NonNull::new(value).expect("Null Pointer in Block"))
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Block) -> bool {
        self.0 == other.0
    }
}

impl Eq for Block {}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Block) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Block {
    fn cmp(&self, other: &Block) -> Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_new() {
        let header = BlockHeader::new(14, 42);
        assert_eq!(42, header.block_size());
        assert_eq!(14, header.pred_block_size());
    }

    #[test]
    fn test_block_header_change_sizes() {
        let mut header = BlockHeader::new(42, 42);
        assert_eq!(42, header.block_size());
        assert_eq!(42, header.pred_block_size());

        header.set_size(10);
        assert_eq!(10, header.block_size());
        assert_eq!(42, header.pred_block_size());

        header.inc_size(2);
        assert_eq!(12, header.block_size());
        assert_eq!(42, header.pred_block_size());

        header.set_pred_size(5);
        assert_eq!(12, header.block_size());
        assert_eq!(5, header.pred_block_size());
    }

    #[test]
    #[should_panic(expected = "Offset is out of bounds")]
    fn test_block_write_panics_if_out_of_bounds() {
        use super::super::address::Address;
        use std::alloc::{alloc, dealloc, Layout};
        use std::mem;

        unsafe {
            // Header + 2 fields
            let size = WORD_SIZE * 3;
            let align = mem::align_of::<usize>();

            let layout = Layout::from_size_align_unchecked(size, align);
            let ptr = NonNull::new_unchecked(alloc(layout)).cast::<usize>();
            let ptr = ptr.as_ptr();

            let mut block = Block::new(ptr, size as HalfWord, 0);
            block.write_at(0, 20);

            let address = Address::from(block);
            assert_eq!(20, *address);

            block.write_at(1, 21);

            let address = Address::from(block);
            assert_eq!(21, *(address + 1));

            // this should panic
            block.write_at(3, 13);

            dealloc(ptr as *mut u8, layout);
        }
    }
}
