use super::address::Address;

use std::iter::Iterator;

pub unsafe trait Traceable {
    /// Mark all contained Traceable Objects
    fn mark(&mut self);
    /// Unmark this Object
    fn unmark(&mut self);
    // /// An iterator used for updating the addresses after moving heap content
    // fn trace(&mut self) -> Box<Iterator<Item = &mut Address>>;
    /// Checks if self is marked
    fn is_marked(&self) -> bool;
}

pub unsafe trait GcRoot<I>
where
    I: Traceable + From<Address> + Into<Address>,
{
    fn children<'a>(&'a mut self) -> Box<Iterator<Item = &'a mut I> + 'a>;
}
