use crate::heap::Heap;
use crate::trace::GcRoot;

pub struct ManagedHeap {
    heap: Heap,
}

impl ManagedHeap {
    pub fn gc(&mut self, roots: &mut [&mut GcRoot]) {
        for traceable in roots.iter_mut().flat_map(|r| r.children()) {
            traceable.mark();
        }

        // TODO iter over used_blocks and free
        // self.heap.iter().filter(|b| b)
    }
}
