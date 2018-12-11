use crate::address::Address;
use std::iter::Iterator;

pub trait Traceable {
    /// Mark all contained Traceable Objects
    fn mark(&mut self);
    /// An iterator used for updating the addresses after moving heap content
    fn trace<'a>(&mut self) -> Box<Iterator<Item = &'a mut Address<'a>>>;
}

pub trait GcRoot {
    fn children<'a>(&mut self) -> Box<Iterator<Item = &'a mut Traceable>>;
}
