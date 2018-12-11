pub trait Traceable {
    /// Marks all Objects, which are referenced by self
    fn mark(&mut self);
}
