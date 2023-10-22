//! Restricted functionality, crossing data access with other structures in an `unsafe` way.

use super::FixedDequeLifos;
use alloc::vec::Vec;
use core::mem;

/// "Front" and "back" RESTRICTED [`Vec`]-s. Each based on the respective part of the
/// [`alloc::collections::VecDeque`] that was a part of [`FixedDequeLifos`] used to create the
/// [`CrossVecPairGuard`] which (in turn) has created this [`CrossVecPair`] instance.
///
/// You MUST NOT exceed the existing capacity of these [`Vec`]-s (neither shrink them, or cause any
/// re-allocation)!
//
// "non_exhaustive" so that clients can't instantiate this. Also, any new fields added in the future
// will work with existing pattern matching/destructuring by the clients.
#[non_exhaustive]
pub struct CrossVecPair<T>(pub Vec<T>, pub Vec<T>);

enum CrossVecPairGuardState<T> {
    /// The two [`Vec`]s correspond to [`FixedDequeLifos::front()`] & [`FixedDequeLifos::back()`],
    /// respectively.
    /// 
    /// Since [`CrossVecPairGuard`]'s (once instantiated) HAS to be "temporarily taken" (see
    /// [`CrossVecPairGuard::new_from_lifos()`]), this initial state can (just as well) contain
    /// [`CrossVecPair`] that will be "temporarily taken" out later (rather than containing the
    /// "ingredients" from the original [`FixedDequeLifos`] or its backing
    /// [`alloc::collections::VecDeque`], and constructing the [`CrossVecPair`] later).
    NotTakenYet(CrossVecPair<T>),
    TakenOut,
    MovedBack
}

/// A wrapper around two [`Vec`]s based on (backed by, shadowing) the same [`FixedDequeLifos`].
///
/// At the end of use, call [`CrossVecPair::forget()`]. Do not let it go out of scope in any other
/// way
/// - otherwise its [`Drop::drop()`] will panic.
//
// After use, the original [`FixedDequeLifos::vec_deque`] would be corrupted if still kept around!
pub struct CrossVecPairGuard<T> {
    state: CrossVecPairGuardState<T>,
    front_ptr: *mut T,
    /// Potentially MORE than the total of capacities of both [`Vec`]-s in `pair`
    /// ([`CrossVecPair`]). Why? because `full_capacity` is the capacity of the original
    /// [`alloc::collections::VecDeque`].
    full_capacity: usize,
    /// Whether the (whole) pair was temporarily "taken" (as if moved out).
    temp_taken: bool,
}
impl<T> From<FixedDequeLifos<T>> for CrossVecPairGuard<T> {
    fn from(lifos: FixedDequeLifos<T>) -> Self {
        let mut vec_deque = lifos.into_vec_deque();
        let (front, back) = vec_deque.as_mut_slices();
        let front_ptr = front.as_mut_ptr();
        let front = unsafe { Vec::from_raw_parts(front_ptr, front.len(), front.len()) };
        let back = unsafe { Vec::from_raw_parts(back.as_mut_ptr(), back.len(), back.len()) };
        let full_capacity = vec_deque.capacity();
        mem::forget(vec_deque);
        Self {
            //pair: Some(CrossVecPair(front, back)),
            front_ptr,
            full_capacity,
            temp_taken: false,
        }
    }
}
impl<T> CrossVecPairGuard<T> {
    /// TODO: Should this be marked as `unsafe`? But: this function itself does NOT cause any
    /// undefined behavior. Its inappropriate use of [`Vec`]-s from a [`CrossVecPair`] "taken" from
    /// a [`CrossVecPairOrigin`] that can lead to undefined behavior.
    ///
    /// If this were marked as `unsafe`, then we should NOT implement [`From`].
    ///
    /// Once you instantiate a [`CrossVecPairOrigin`] (which is possible ony with this function),
    /// you MUST take the pair out with [CrossVecPairOrigin::temp_take()], and then move it back
    /// with [CrossVecPairOrigin::move_join_into()].
    ///
    /// You MUST not let a [`CrossVecPairOrigin`] instance go out of scope without taking the pair
    /// out & then putting it back and discarding as per above.
    #[must_use]
    pub fn new_from_lifos(fixed_deque_lifos: FixedDequeLifos<T>) -> Self {
        fixed_deque_lifos.into()
    }

    /// "Take" the (whole). Like "moving out".
    ///
    /// We need this temporary "move out" ability, so that we can then transform the [`Vec`]
    /// into[`alloc::collections::VecDeque`] in the next deeper recursion level. We do it with
    /// <https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html#impl-From%3CVec%3CT,+A%3E%3E-for-VecDeque%3CT,+A%3E>,
    /// which takes the [`Vec`] by value (move).
    ///
    /// Once you're finished using the pair, undo this with [CrossVecPairOrigin::move_join_into()].
    #[must_use]
    pub fn temp_take(&mut self) -> CrossVecPair<T> {
        debug_assert!(!self.temp_taken, "Already 'temporarily taken.'");
        self.temp_taken = true;

        fn cross_vec<T>(v: &mut Vec<T>) -> Vec<T> {
            let len = v.len();
            let capacity = v.capacity();
            unsafe { Vec::from_raw_parts(v.as_mut_ptr(), len, capacity) }
        }
        let current = self.pair.as_mut().unwrap();
        CrossVecPair(cross_vec(&mut current.0), cross_vec(&mut current.1))
    }

    /// Safely discard the given [`CrossVecPair`] that was "taken" from this [`CrossVecPairOrigin`]
    /// instance, and discard this this [`CrossVecPairOrigin`] instance itself.
    ///
    /// Check that the parameter `pair` are [`Vec`]s based on this [`CrossVecPairOrigin`] instance.
    /// Then "move" the pair back, move any leftover items from the "back" (right) side to the
    /// "front" (left) side. Then consume this [`CrossVecPairOrigin`] instance (without releasing
    /// any memory), and transform it back to a single [`Vec`] (but NOT back to a
    /// [`alloc::collections::VecDeque`], since in such a stage this crate doesn't need it as
    /// [`alloc::collections::VecDeque`] anymore. The result [`Vec`]) will have its `capacity` same
    /// as the original [`alloc::collections::VecDeque`].
    ///
    /// You MUST call this before the instance (if you "took" a [CrossVecPair] from it) before this
    /// ([`CrossVecPairOrigin`] instance) goes out of scope.
    ///
    /// You don't have to re-use this function's result [`Vec`]. But it's advantageous to re-use it,
    /// so as to minimize reallocation (which is this crate's main purpose).
    #[must_use]
    pub fn move_join_into(mut self, pair: CrossVecPair<T>) -> Vec<T> {
        debug_assert!(self.temp_taken, "Not 'temporarily taken.'");
        self.temp_taken = false;

        let current = self.pair.as_ref().unwrap();
        // We do NOT compare length, since it may have drifted to be different.
        debug_assert_eq!(pair.0.as_ptr(), current.0.as_ptr());
        debug_assert_eq!(pair.1.as_ptr(), current.1.as_ptr());
        self.pair = Some(pair);
        let pair = self.pair.take();
        let CrossVecPair(front, back) = pair.unwrap();
        mem::forget(front);
        mem::forget(back);
        todo!()
    }
}
impl<T> Drop for CrossVecPairGuard<T> {
    fn drop(&mut self) {
        debug_assert!(!self.temp_taken, "'Temporarily taken.'");
        debug_assert!(self.pair.is_none());
    }
}
