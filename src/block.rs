use std::cmp::Ordering;
use std::fmt;
use std::ptr::NonNull;

/// The first field in a block of memory.
/// This type is treated as a u32, even though it's an usize.
/// Contains the size of the block in its first 2 bytes.
#[derive(Copy, Clone)]
pub struct BlockHeader(usize);

impl BlockHeader {
    pub fn new(size: u16) -> Self {
        BlockHeader((u32::from(size) << 16) as usize)
    }

    pub fn block_size(self) -> u16 {
        (self.0 as u32 >> 16) as u16
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
    pub fn new(ptr: *mut usize, size: u16) -> Self {
        let header = BlockHeader::new(size);
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
    pub fn size(self) -> u16 {
        unsafe { self.0.as_ref().block_size() }
    }

    /// Splits the block by inserting a new header at self + size
    pub unsafe fn split_after(self, size: u16) -> (Block, Block) {
        let current_size = self.size();
        assert!(current_size > size, "size too big");

        let second_size = current_size - size;

        let ptr = self.0.as_ptr() as *mut usize;

        let second_ptr = ptr.add(size as usize);
        *second_ptr = BlockHeader::new(second_size).into();
        let second = Block(NonNull::new_unchecked(second_ptr as *mut BlockHeader));

        *ptr = BlockHeader::new(size).into();

        (self, second)
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Block ({})", self.size())
    }
}

impl Into<NonNull<BlockHeader>> for Block {
    fn into(self) -> NonNull<BlockHeader> {
        self.0
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
        let header = BlockHeader::new(42);
        assert_eq!(42, header.block_size());
    }
}
