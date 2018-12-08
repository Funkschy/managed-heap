pub trait Finalize {
    fn finalize(&self) {}
}

pub trait Trace {
    fn trace(&self);
}
