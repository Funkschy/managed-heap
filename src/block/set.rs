use super::Block;
use crate::types::HalfWord;

#[derive(Default)]
pub struct BlockSet(Vec<Block>);

impl BlockSet {
    pub fn from_raw(ptr: *mut usize, size: HalfWord) -> Self {
        let mut block_vec = Self::default();

        let block = Block::new(ptr, size, 0);
        block_vec.add_block(block);

        block_vec
    }
}

impl BlockSet {
    pub fn contains(&self, block: Block) -> bool {
        self.0.binary_search(&block).is_ok()
    }

    pub fn iter<'a>(&'a self) -> Box<Iterator<Item = &Block> + 'a> {
        Box::new(self.0.iter())
    }
}

impl BlockSet {
    pub fn add_block(&mut self, block: Block) {
        let index = match self.0.binary_search(&block) {
            Ok(index) => index,
            Err(index) => index,
        };
        self.0.insert(index, block);
    }

    pub fn get_block(&mut self, min_size: HalfWord) -> Option<Block> {
        let block = self.0.iter().find(|b| b.size() >= min_size);
        if let Some(b) = block {
            let b = *b;
            let index = self.0.binary_search(&b).ok()?;
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    pub fn remove_block(&mut self, block: Block) {
        let index = self.0.binary_search(&block);
        if let Ok(i) = index {
            self.0.remove(i);
        }
    }
}

impl BlockSet {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
