#![no_std]
#![cfg_attr(feature = "_internal_use_allocator_api", feature(allocator_api))]
#![allow(incomplete_features)]
#![cfg_attr(not(feature = "nightly_lazy_type_alias"), allow(type_alias_bounds))]
#![cfg_attr(feature = "nightly_lazy_type_alias", feature(lazy_type_alias))]
#![cfg_attr(feature = "nightly_strict_provenance", feature(strict_provenance))]

extern crate alloc;

use alloc::vec::Vec; //TODO calloc
use alloc::vec::Vec as StdVec;
use calloc::{Allocator, Global};
use core::{mem, ops::Deref};
//use cross;
use lifos::FixedDequeLifos;

pub mod calloc;
mod cross;
mod lifos;

#[cfg(test)]
mod test {
    #[test]
    fn convert_not_invoking_drop() {}
}

/// Array of two mutable [`Vec`] references.
///
/// Handling [`core::mem::MaybeUninit`] directly could be a little bit more efficient, but too
/// risky.
///
/// Using [`alloc::collections::vec_deque::VecDeque`] would make the implementation easier, but less
/// efficient (because its lower bound is not necessarily `0`, but movable around the ring buffer.
/// Also, [`Vec`] is more common.
pub type StorePair<T> = [Vec<T>; 2];

pub type InputStorePair<T> = (Vec<T>, StorePair<T>);
#[cfg(feature = "nightly_accept_custom_alloc")]
pub type OutputPair<T, A: Allocator = Global> = (Vec<T>, Storage<T, A>);
#[cfg(not(feature = "nightly_accept_custom_alloc"))]
pub type OutputPair<T> = (Vec<T>, Storage<T>);

/// The [`bool`] part (complete) indicates whether we've handled (sorted & consumed) all items.
///
/// For example, in sorting methods that take a closure which returns [`bool`] indicating whether to
/// continue (sorting & consuming), complete is true if such a closure returns `true` for all items.
///
/// However, we could have completed all sorting and consuming of the (sorted) items, even if
/// "complete" part is `false`. In such an instance "complete" would be indicated as `false` only at
/// the consumption of the very last (highest) sorted item, when this "complete" being false doesn't
/// make any difference.
type InputStorePairCompletion<T> = (InputStorePair<T>, bool);

/// This exists, so that we don't mix up [`Vec`] parts of [`InputStoreSingleCompletion`].
#[repr(transparent)]
struct StoreSingle<T>(Vec<T>);
impl<T> Deref for StoreSingle<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.0
    }
}

/// This could be "implemented" to be the same as [`StorePair`]. But by being different we avoid
/// mistaking them (and we don't need to introduce any newtype wrapper).
///
/// For the [`bool`] part (complete) see [`InputStorePairCompletion`].
type InputStoreSingleCompletion<T> = (Vec<T>, StoreSingle<T>, bool);

/// For ensuring we use a result from closures.
#[must_use]
#[repr(transparent)]
struct MustUse<T>(T);

#[inline(always)]
fn debug_assert_empty<T>(store_pair: &StorePair<T>) {
    debug_assert!(store_pair[0].is_empty());
    debug_assert!(store_pair[1].is_empty());
}

/// Assert that all [`Vec`] in `storage` have sufficient capacity (equal to, or greater than,
/// `capacity`).
#[inline(always)]
fn debug_assert_capacity<T>(store_pair: &StorePair<T>, capacity: usize) {
    debug_assert!(store_pair[0].capacity() >= capacity);
    debug_assert!(store_pair[1].capacity() >= capacity);
}

/// Generate a new closure whose result is `#[must_use]`. Should be zero-cost.
#[inline(always)]
fn make_consume_closure_must_use_result<T, CONSUME>(
    mut consume: CONSUME,
) -> impl FnMut(usize, T) -> MustUse<bool>
where
    CONSUME: FnMut(usize, T) -> bool,
{
    move |idx, value| MustUse(consume(idx, value))
}

// ------ Storage
#[cfg(feature = "nightly_accept_custom_alloc")]
/// Storage, ideally to be re-used by the client.
#[must_use]
#[repr(transparent)]
pub struct Storage<T, A: Allocator = Global>(StdVec<T, A>);

#[cfg(feature = "nightly_accept_custom_alloc")]
impl<T, A: Allocator> From<StdVec<T, A>> for Storage<T, A> {
    fn from(v: StdVec<T, A>) -> Self {
        Self(v)
    }
}

#[cfg(feature = "nightly_accept_custom_alloc")]
impl<T, A: Allocator> Storage<T, A> {
    pub fn into_vec(self) -> StdVec<T, A> {
        self.0
    }
}
//--

#[cfg(not(feature = "nightly_accept_custom_alloc"))]
/// Storage, ideally to be re-used by the client.
#[must_use]
#[repr(transparent)]
pub struct Storage<T>(StdVec<T>);

#[cfg(not(feature = "nightly_accept_custom_alloc"))]
impl<T> From<StdVec<T>> for Storage<T> {
    fn from(v: StdVec<T>) -> Self {
        Self(v)
    }
}

#[cfg(not(feature = "nightly_accept_custom_alloc"))]
impl<T> Storage<T> {
    pub fn into_vec(self) -> StdVec<T> {
        self.0
    }
}
// ---------- end of: Storage

//  ---------- qsort_idx(..)
#[cfg(feature = "nightly_accept_custom_alloc")]
#[must_use]
pub fn qsort_idx_NEW<T, CONSUME, A: Allocator>(
    input: Vec<T, A>,
    storage: Storage<T, A>,
    consume: &mut CONSUME,
) -> OutputPair<T, A>
where
    T: Ord,
    CONSUME: FnMut(usize, T) -> bool,
{
    todo!()
}
//  --

#[cfg(not(feature = "nightly_accept_custom_alloc"))]
#[must_use]
pub fn qsort_idx_NEW<T, CONSUME>(
    input: Vec<T>,
    storage: Storage<T>,
    consume: &mut CONSUME,
) -> OutputPair<T>
where
    T: Ord,
    CONSUME: FnMut(usize, T) -> bool,
{
    todo!()
}
//  ---------- end of: qsort_idx(..)

///
/// - `consume`: A handler (consumer) closure to consume the items in sorted order, one by one. It
///   returns `true` to continue, or `false` once finished.
///   - Its first param of type `usize` is a 0-based index/sequential order number of the next
///    output item to be consumed (increasing by one per each output item; not related to the item's
///    position in `input`).
///
/// You don't really have to use the result (3 [`Vec`]-tors). But if you can re-use them, you save
/// allocation & de-allocation. The [`Vec`]-tors in the returned array contain `input` first and
/// then the 2 [`Vec`]-tors from `storage`.
///
/// There are no guarantees about position/order of any items left in the result [`InputStorePair`],
/// other that they are all items (and only those items) that haven't been consumed (passed to
/// `consume`).
//
// Not part of the contract/API: This starts removing items (the pivot) from `input` from its end,
// to avoid shuffling.
#[must_use]
pub fn qsort_idx<T, CONSUME>(
    input: Vec<T>,
    store_pair: StorePair<T>,
    consume: &mut CONSUME,
) -> InputStorePair<T>
where
    T: Ord,
    CONSUME: FnMut(usize, T) -> bool,
{
    //consume_must_use_result::<T, _>(unsafe { mem::transmute(consume) });
    debug_assert_empty(&store_pair);
    let input_initial_len = input.len();
    debug_assert_capacity(&store_pair, input_initial_len);

    let mut consumed_so_far = 0usize;
    let mut consume = make_consume_closure_must_use_result(consume);
    let ((input, store_pair), complete) =
        part_store_pair_idx(input, store_pair, &mut consume, &mut consumed_so_far);
    if complete {
        debug_assert!(input.is_empty());
        debug_assert_empty(&store_pair);
        debug_assert_eq!(consumed_so_far, input_initial_len);
    } else {
        debug_assert_eq!(
            consumed_so_far + input.len() + store_pair[0].len() + store_pair[1].len(),
            input_initial_len
        );
    }
    (input, store_pair)
}

/// - `next_out_seq_idx`: 0-based index/sequential order number of the next output item to be
///    consumed (increasing by one per each output item; not related to the item's position in
///    `input`).
#[must_use]
fn part_store_pair_idx<T, CONSUME>(
    mut input: Vec<T>,
    store_pair: StorePair<T>,
    consume: &mut CONSUME,
    consumed_so_far: &mut usize,
) -> InputStorePairCompletion<T>
where
    T: Ord,
    CONSUME: FnMut(usize, T) -> MustUse<bool>,
{
    debug_assert_empty(&store_pair);
    debug_assert_capacity(&store_pair, input.len());
    if input.is_empty() {
        return ((input, store_pair), true);
    }
    let pivot = input.pop().unwrap();
    let [mut lower_side, mut greater_equal_side] = store_pair;

    while !input.is_empty() {
        let value = input.pop().unwrap();
        if value < pivot {
            lower_side.push(value);
        } else {
            greater_equal_side.push(value);
        }
    }
    debug_assert!(input.is_empty());

    // We reuse `input` and `lower`, consuming them, then returning (moving) them back.
    let (lower_side, StoreSingle(mut input), complete) =
        part_store_single_idx(lower_side, StoreSingle(input), consume, consumed_so_far);
    if complete {
        debug_assert!(lower_side.is_empty());
        debug_assert!(input.is_empty());
    } else {
        input.push(pivot);
        return ((input, [lower_side, greater_equal_side]), false);
    }

    let complete = consume(*consumed_so_far, pivot);
    *consumed_so_far += 1;
    if !complete.0 {
        return ((input, [lower_side, greater_equal_side]), false);
    }

    let (greater_equal_side, StoreSingle(input), complete) = part_store_single_idx(
        greater_equal_side,
        StoreSingle(input),
        consume,
        consumed_so_far,
    );
    if complete {
        debug_assert!(greater_equal_side.is_empty());
        debug_assert!(input.is_empty());
    }
    ((input, [lower_side, greater_equal_side]), complete)
}

#[must_use]
fn part_store_single_idx<T, CONSUME>(
    mut input: Vec<T>,
    store_single: StoreSingle<T>,
    mut consume: &mut CONSUME,
    consumed_so_far: &mut usize,
) -> InputStoreSingleCompletion<T>
where
    T: Ord,
    CONSUME: FnMut(usize, T) -> MustUse<bool>,
{
    debug_assert!(store_single.is_empty());
    // We reuse `input`: splitting it (by consuming it), then receiving it back in parts,
    // re-constructing it and returning (moving) it back.
    //
    // We also consume, receive & return `sub`.
    match input.len() {
        0 => (input, store_single, true),
        1 => {
            let complete = consume(*consumed_so_far, input.pop().unwrap());
            *consumed_so_far += 1;
            (input, store_single, complete.0)
        }
        2 => {
            // Let's save splitting & reconstructing the Storage vectors: sort 2 items manually.
            let mut one = input.pop().unwrap();
            let mut two = input.pop().unwrap();
            if one > two {
                mem::swap(&mut one, &mut two);
            }
            let complete = consume(*consumed_so_far, one);
            *consumed_so_far += 1;
            if !complete.0 {
                input.push(two);
                return (input, store_single, false);
            }
            let complete = consume(*consumed_so_far, two);
            *consumed_so_far += 1;
            (input, store_single, complete.0)
        }
        input_len => {
            let store_orig_capacity = store_single.capacity();

            // Set the capacity to any possible maximum, but not more - so that we catch any errors
            // a.s.a.p.
            //
            // Oh!:  2*side_len may be MORE than store_single.capacity!
            let store_pair = unsafe { split_vec(store_single.0, input_len, input_len) };

            let ((input, store_pair), complete) =
                part_store_pair_idx(input, store_pair, consume, consumed_so_far);
            if !complete {
                todo!()
            } else {
                debug_assert!(input.is_empty());
            }

            let store_single = unsafe { join_vecs(store_pair) };

            debug_assert_eq!(store_single.capacity(), store_orig_capacity);
            (input, StoreSingle(store_single), complete)
        }
    }
}

/// Similar to [`[T]::split_at_mut()`]:
/// <https://doc.rust-lang.org/nightly/core/primitive.slice.html#method.split_at_mut>. But, NOT like
/// [`Vec::split_at(&self,usize)`], because that allocates one of the two [`Vec`]-tors and moves its
/// part of the data!
///
/// The result contains 2 [`Vec`]-tors of capacity equal to `capacity_one, capacity_two`,
/// respectively, and a [`usize`] original capacity (which may be larger than
/// `capacity_one+capacity_two`).
///
/// Thanks to <https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html#guarantees>
/// - "Vec will never automatically shrink itself, even if completely empty."
/// - "push and insert will never (re)allocate if the reported capacity is sufficient"
///
/// Do NOT let the result 2 [`Vec`]-tors [`Drop::drop`] automatically. Hence, do NOT let the result
/// leave this module. Instead, pass them both to [`join_vecs`].
#[must_use]
unsafe fn split_vec<T>(store: Vec<T>, capacity_one: usize, capacity_two: usize) -> StorePair<T> {
    debug_assert!(capacity_one + capacity_two <= store.capacity());
    mem::forget(store);

    todo!()
}

/// Reconstruct a [`Vec`] from two split "subvectors". You must use this before you want to
/// [`Drop::drop`] it (them) automatically, or before you pass it outside this module (for re-use).
///
/// Only pass two adjacent [`Vec`]-tors returned from the same call to [`split_vec`].
#[must_use]
unsafe fn join_vecs<T>(vecs: StorePair<T>) -> Vec<T> {
    todo!()
    //result
}
/*
pub fn qsort_len<T: Copy, S: FnMut(usize, T)>(items: &mut [T], store: S, len: usize) {}

pub fn qsort_sub<T: Copy, S: FnMut(usize, T), THRESHOLD: FnMut(T, T) -> T>(
    items: &mut [T],
    store: S,
    threshold: THRESHOLD,
) {
}

pub fn qsort_with_acc<T: Copy, ACU, S: FnMut(usize, T), ACCUMULATE: FnMut(ACU, T) -> ACU>(
    items: &mut [T],
    store: S,
    accumulate: ACCUMULATE,
) {
}

fn part_and_collect<T: Copy, COLL, C: FnMut(COLL, T) -> COLL, F: FnMut(usize, T) -> bool>(
    items: &mut [T],
    f: F,
    next_result_item_idx: usize,
) {
}

#[cfg(test)]
mod tests {
}
*/
