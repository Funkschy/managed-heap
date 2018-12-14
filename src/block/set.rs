use super::Block;
use crate::types::HalfWord;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct BlockSet(BTreeSet<Block>);

impl BlockSet {
    pub fn from_raw(ptr: *mut usize, size: HalfWord) -> Self {
        let mut block_vec = BlockSet::default();

        let block = Block::new(ptr, size, 0);
        block_vec.add_block(block);

        block_vec
    }
}

impl BlockSet {
    pub fn contains(&self, block: Block) -> bool {
        self.0.contains(&block)
    }

    pub fn iter<'a>(&'a self) -> Box<Iterator<Item = &Block> + 'a> {
        Box::new(self.0.iter())
    }
}

impl BlockSet {
    pub fn add_block(&mut self, block: Block) {
        self.0.insert(block);
    }

    pub fn get_block(&mut self, min_size: HalfWord) -> Option<Block> {
        let block = self.0.iter().find(|b| b.size() >= min_size);
        if let Some(b) = block {
            let b = *b;
            self.0.take(&b)
        } else {
            None
        }
    }

    pub fn remove_block(&mut self, block: Block) {
        self.0.remove(&block);
    }
}

impl BlockSet {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
