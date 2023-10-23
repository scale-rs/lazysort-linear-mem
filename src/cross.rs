//! Restricted functionality, crossing data access with other structures in an `unsafe` way.
//!
//! (Yes, there is "stuttering" (type names here start with "Cross", which is also in the package
//! name). Idiomatic way to use types is to import them. Then there is no "stuttering".)

use crate::lifos::FixedDequeLifos;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter, Result as FmtResult};
use core::mem;

#[cfg(test)]
mod cross_tests;

#[cfg(not(feature = "nightly_guard_cross_alloc"))]
pub type CrossVec<T> = Vec<T>;
#[cfg(all(
    feature = "nightly_guard_cross_alloc",
    not(feature = "nightly_guard_cross_cleanup")
))]
// TODO custom Alloc
pub type CrossVec<T> = Vec<T>;
#[cfg(feature = "nightly_guard_cross_cleanup")]
// TODO custom Alloc with cleanup check
pub type CrossVec<T> = Vec<T>;

/// "Front" and "back" RESTRICTED [`Vec`]-s (in this order). Each based on the respective part of
/// the [`alloc::collections::VecDeque`] that was a part of [`FixedDequeLifos`] used to create the
/// [`CrossVecPairGuard`] which (in turn) has created this [`CrossVecPair`] instance.
///
/// You MUST NOT exceed the existing capacity of these [`Vec`]-s (neither shrink them, or cause any
/// re-allocation)!
///
/// "non_exhaustive" so that
/// - clients can't instantiate this. Also,
/// - any new fields added in the future will work with existing pattern matching/destructuring by
///   the clients.
#[non_exhaustive]
#[derive(Debug)]
pub struct CrossVecPair<T>(pub CrossVec<T>, pub CrossVec<T>);

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
    #[cfg(not(feature = "nightly_guard_cross_cleanup"))]
    TakenOut,
    #[cfg(feature = "nightly_guard_cross_cleanup")]
    /// TODO a field with 2x Arc - one per Vec.
    ///
    /// Using [Arc], instead of [Rc], in case [`CrossVecPair`] or any of its [`Vec`]-s is sent to a
    /// different thread and gets dropped there.
    TakenOut,
    MovedBack,
}
impl<T> CrossVecPairGuardState<T> {
    fn is_not_taken_yet(&self) -> bool {
        matches!(self, CrossVecPairGuardState::NotTakenYet(_))
    }
    fn is_taken_out(&self) -> bool {
        matches!(self, CrossVecPairGuardState::TakenOut)
    }
    fn is_moved_back(&self) -> bool {
        matches!(self, CrossVecPairGuardState::MovedBack)
    }
}
impl<T> Debug for CrossVecPairGuardState<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NotTakenYet(_) => f.write_str("Self::NotTakenYet(_)"),
            Self::TakenOut => f.write_str("Self::TakenOut"),
            Self::MovedBack => f.write_str("Self::MovedBack"),
        }
    }
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
    orig_front_len: usize,
    orig_back_len: usize,
    front_ptr: *mut T,
    back_ptr: *mut T,
    /// Potentially MORE than the total of capacities of both [`Vec`]-s "temporarily taken out" in
    /// the generated [`CrossVecPair`]. Why? because `full_capacity` is the capacity of the original
    /// [`alloc::collections::VecDeque`].
    full_capacity: usize,
}
impl<T> From<FixedDequeLifos<T>> for CrossVecPairGuard<T> {
    fn from(lifos: FixedDequeLifos<T>) -> Self {
        let mut vec_deque = lifos.into_vec_deque();
        let (front, back) = vec_deque.as_mut_slices();

        let orig_front_len = front.len();
        let orig_back_len = back.len();

        let front_ptr = front.as_mut_ptr();
        let back_ptr = back.as_mut_ptr();

        let front = unsafe { Vec::from_raw_parts(front_ptr, orig_front_len, orig_front_len) };
        let back = unsafe { Vec::from_raw_parts(back_ptr, orig_back_len, orig_back_len) };

        let full_capacity = vec_deque.capacity();

        mem::forget(vec_deque);
        Self {
            state: CrossVecPairGuardState::NotTakenYet(CrossVecPair(front, back)),
            orig_front_len,
            orig_back_len,
            front_ptr,
            back_ptr,
            full_capacity,
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
    /// with [CrossVecPairOrigin::move_back_join_into()]. Regardless of whether you use the pair or
    /// not!
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
    /// Once you're finished using the [`CrossVecPair`], undo this with
    /// [CrossVecPairOrigin::move_back_join_into()].
    #[must_use]
    pub fn temp_take(&mut self) -> CrossVecPair<T> {
        // self.state does get checked later in this function, too - and even in release.
        //
        // But, that's after a mutation ot self (because we have to move self.state out of self,
        // since it cannot be Clone/Copy). Hence checking this double-check.
        debug_assert!(self.state.is_not_taken_yet(), "Expecting the CrossVecPair NOT to be taken out yet. But CrossVecPairGuard::state is: {:?}.", self.state);

        let previous_state = mem::replace(&mut self.state, CrossVecPairGuardState::TakenOut);
        let CrossVecPairGuardState::NotTakenYet(pair) = previous_state else {
            panic!("Expecting the CrossVecPair NOT to be taken out yet. But CrossVecPairGuard::state is: {:?}.", self.state);
            // It gets checked by the following,
        };
        pair
        /*
            match self.state {
                CrossVecPairGuardState::NotTakenYet(pair) => {
                    self.state = CrossVecPairGuardState::TakenOut;
                    pair
                }
                _ => panic!("Expecting the CrossVecPair NOT to be taken out yet. But CrossVecPairGuard::state is: {:?}.", self.state)
            }
        */
    }

    #[inline(always)]
    fn debug_assert_consistent(&self, pair: &CrossVecPair<T>) {}

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
    pub fn move_back_join_into(mut self, pair: CrossVecPair<T>) -> Vec<T> {
        debug_assert!(
            self.state.is_taken_out(),
            "Expecting CrossVecPairGuardState to be 'taken out', but it's: {:?}.",
            self.state
        );
        // TODO should these asserts be run also in release?
        debug_assert_eq!(pair.0.as_ptr(), self.front_ptr);
        debug_assert_eq!(pair.1.as_ptr(), self.back_ptr);
        debug_assert!(pair.0.len() <= self.orig_front_len);
        debug_assert!(pair.1.len() <= self.orig_back_len);
        debug_assert!(pair.0.capacity() == self.orig_front_len);
        debug_assert!(pair.1.capacity() == self.orig_back_len);
        let CrossVecPair(front, back) = pair;
        mem::forget(front);
        mem::forget(back);

        self.state = CrossVecPairGuardState::MovedBack;
        todo!()
    }
}
impl<T> Drop for CrossVecPairGuard<T> {
    fn drop(&mut self) {
        debug_assert!(
            self.state.is_moved_back(),
            "Expecting the CrossVecPair to be moved back, but it's: {:?}.'",
            self.state
        );
    }
}
