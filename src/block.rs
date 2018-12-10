use std::cmp::Ordering;
use std::fmt;
use std::mem;
use std::ptr::NonNull;

/// The first field in a block of memory.
/// This type is treated as a u32, even though it's an usize.
/// Contains the size of the previous block in its first 2 bytes and its own
/// in the last 2 bytes.
#[derive(Copy, Clone)]
pub struct BlockHeader(usize);

impl BlockHeader {
    const PRED_FLAG: usize = 0xFFFF_0000;
    const SIZE_FLAG: usize = 0x0000_FFFF;

    pub fn new(pred_size: u16, size: u16) -> Self {
        let pred = u32::from(pred_size) << 16;
        let own = u32::from(size);
        let word = pred | own;

        BlockHeader(word as usize)
    }

    pub fn block_size(self) -> u16 {
        self.0 as u16
    }

    pub fn pred_block_size(self) -> u16 {
        (self.0 as u32 >> 16) as u16
    }
}

impl BlockHeader {
    fn inc_size(&mut self, value: u16) {
        let size = u32::from(self.block_size() + value);
        self.0 = (self.0 & BlockHeader::PRED_FLAG) + size as usize;
    }

    fn set_size(&mut self, value: u16) {
        self.0 = (self.0 & BlockHeader::PRED_FLAG) + value as usize;
    }

    fn set_pred_size(&mut self, value: u16) {
        let size = (u32::from(value) << 16) as usize;
        let cleared = self.0 & BlockHeader::SIZE_FLAG;
        self.0 = size | cleared;
    }
}

impl PartialOrd for BlockHeader {
    fn partial_cmp(&self, other: &BlockHeader) -> Option<Ordering> {
        Some(self.block_size().cmp(&other.block_size()))
    }
}

impl Ord for BlockHeader {
    fn cmp(&self, other: &BlockHeader) -> Ordering {
        self.block_size().cmp(&other.block_size())
    }
}

impl PartialEq for BlockHeader {
    fn eq(&self, other: &BlockHeader) -> bool {
        self.block_size() == other.block_size()
    }
}

impl Eq for BlockHeader {}

impl Into<usize> for BlockHeader {
    fn into(self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct Block(NonNull<BlockHeader>);

impl Block {
    pub fn new(ptr: *mut usize, size: u16, pred_size: u16) -> Self {
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
    pub fn write_at(&mut self, offset: u16, value: usize) {
        assert!(offset % (mem::size_of::<usize>() as u16) == 0);
        assert!(offset < self.size());

        unsafe {
            // add one to offset, to skip header
            *(self.0.as_ptr() as *mut usize).add(1 + offset as usize) = value;
        }
    }

    pub fn inc_size(&mut self, value: u16) {
        unsafe {
            self.0.as_mut().inc_size(value);
        }
    }

    pub fn set_size(&mut self, value: u16) {
        unsafe {
            self.0.as_mut().set_size(value);
        }
    }

    pub fn set_pred_size(&mut self, value: u16) {
        unsafe {
            self.0.as_mut().set_pred_size(value);
        }
    }
}

impl Block {
    pub fn size(self) -> u16 {
        unsafe { self.0.as_ref().block_size() }
    }

    pub fn pred_size(self) -> u16 {
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
    pub unsafe fn split_after(self, size: u16) -> (Block, Block) {
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
        self.size() == other.size()
    }
}

impl Eq for Block {}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Block) -> Option<Ordering> {
        Some(self.size().cmp(&other.size()))
    }
}

impl Ord for Block {
    fn cmp(&self, other: &Block) -> Ordering {
        self.size().cmp(&other.size())
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
}
