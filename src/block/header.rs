use crate::types::{HalfWord, Word};
use std::cmp::Ordering;
use std::mem;

/// The first field in a block of memory.
/// Contains the size of the previous block in its first 2 bytes and its own
/// in the last 2 bytes.
#[derive(Copy, Clone)]
pub struct BlockHeader(usize);

impl BlockHeader {
    #[cfg(target_pointer_width = "64")]
    const PRED_FLAG: usize = 0xFFFF_FFFF_0000_0000;
    #[cfg(target_pointer_width = "32")]
    const PRED_FLAG: usize = 0xFFFF_0000;

    const SIZE_FLAG: usize = !BlockHeader::PRED_FLAG;

    const SHIFT: usize = mem::size_of::<HalfWord>() * 8;

    pub fn new(pred_size: HalfWord, size: HalfWord) -> Self {
        let pred = Word::from(pred_size) << BlockHeader::SHIFT;
        let own = Word::from(size);
        let word = pred | own;

        BlockHeader(word as usize)
    }

    pub fn block_size(self) -> HalfWord {
        self.0 as HalfWord
    }

    pub fn pred_block_size(self) -> HalfWord {
        (self.0 as Word >> BlockHeader::SHIFT) as HalfWord
    }
}

impl BlockHeader {
    pub fn inc_size(&mut self, value: HalfWord) {
        let size = Word::from(self.block_size() + value);
        self.0 = (self.0 & BlockHeader::PRED_FLAG) + size as usize;
    }

    pub fn set_size(&mut self, value: HalfWord) {
        self.0 = (self.0 & BlockHeader::PRED_FLAG) + value as usize;
    }

    pub fn set_pred_size(&mut self, value: HalfWord) {
        let size = (Word::from(value) << BlockHeader::SHIFT) as usize;
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
