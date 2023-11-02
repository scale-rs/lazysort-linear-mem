pub trait Lifos<T> {
    fn has_to_push_left_first() -> bool;
}

// - TODO no-alloc-friendly "SliceDeque" struct
// - TODO when Storage is backed by an array, make the array size a const generic
// - TODO a trait and an adapter for VecDeque

#[cfg(feature = "alloc")]
pub mod lifos_vec;
