use crate::block::Block;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct BlockVec(BTreeSet<Block>);

impl BlockVec {
    pub fn from_raw(ptr: *mut usize, size: u16) -> Self {
        let mut block_vec = BlockVec::default();

        println!("{}", size);
        let block = Block::new(ptr, size);
        block_vec.add_block(block);

        block_vec
    }
}

impl BlockVec {
    pub fn add_block(&mut self, block: Block) {
        self.0.insert(block);
    }

    pub fn get_block(&mut self, min_size: u16) -> Option<Block> {
        println!("{:?}", self.0);

        let block = self.0.iter().find(|b| b.size() >= min_size);
        if let Some(b) = block {
            let b = *b;
            self.0.take(&b)
        } else {
            None
        }
    }
}
