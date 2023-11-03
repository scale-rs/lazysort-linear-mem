pub trait Lifos<T> {
    fn has_to_push_left_first() -> bool;
    fn push_left(&mut self, value: T);
    fn push_right(&mut self, value: T);
    /// How many items on the right.
    fn right(&self) -> usize;
    /// How many items on the left.
    fn left(&self) -> usize;
}

// - TODO no-alloc-friendly "SliceDeque" struct
// - TODO when Storage is backed by an array, make the array size a const generic
// - TODO a trait and an adapter for VecDeque

#[cfg(feature = "alloc")]
pub mod lifos_vec;
