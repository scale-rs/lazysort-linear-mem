#![no_std]
#![feature(vec_into_raw_parts)]
extern crate alloc;

use alloc::vec::Vec;
use core::mem;

/// Array of two mutable [`Vec`] references.
///
/// Handling [`core::mem::MaybeUninit`] directly could be a little bit more efficient, but too
/// risky.
///
/// Using [`alloc::collections::vec_deque::VecDeque`] would make the implementation easier, but less
/// efficient (because its lower bound is not necessarily `0`, but movable around the ring buffer.
/// Also, [`Vec`] is more common.
pub type StorePair<T> = [Vec<T>; 2];

/// The [`bool`] part (completion) indicates whether we've handled (sorted & consumed) all items.
/// For example, in sorting methods that take a closure which returns [`bool`] indicating whether to
/// continue (sorting & consuming), completion is true if such a closure returns `true` for all
/// items.
type InputStorePairCompletion<T> = (Vec<T>, StorePair<T>, bool);

/// This could be "implemented" to be the same as [`StorePair`]. But by being different we avoid
/// mistaking them (and we don't need to introduce any newtype wrapper).
///
/// For the [`bool`] part (completion) see [`InputStorePairCompletion`].
type InputStoreSingleCompletion<T> = (Vec<T>, Vec<T>, bool);

/// For ensuring we use a result from closures.
#[must_use]
#[repr(transparent)]
struct MustUse<T>(T);

#[inline(always)]
fn debug_assert_empty<T>(store_pair: &StorePair<T>) {
    debug_assert!(store_pair[0].is_empty());
    debug_assert!(store_pair[1].is_empty());
}

/// Assert that all [`Vec`] in `storage` have sufficient capacity (equal to, or greater than, `capacity`).
#[inline(always)]
fn debug_assert_capacity<T>(store_pair: &StorePair<T>, capacity: usize) {
    debug_assert!(store_pair[0].capacity() >= capacity);
    debug_assert!(store_pair[1].capacity() >= capacity);
}

/// Generate a new closure whose result is `#[must_use]`. Should be zero-cost.
#[inline(always)]
fn make_consume_closure_must_use_result<T, CONSUME>(
    consume: CONSUME,
) -> impl Fn(usize, T) -> MustUse<bool>
where
    CONSUME: Fn(usize, T) -> bool,
{
    move |idx, value| MustUse(consume(idx, value))
}

///
/// - `consume`: A handler (consumer) closure to consume the items in sorted order, one by one. It
///   returns `true` to continue, or `false` once finished.
///
/// You don't really have to use the result (3 [`Vec`]-tors). But if you can re-use them, you save
/// allocation & de-allocation. The [`Vec`]-tors in the returned array contain `input` first and
/// then the 2 [`Vec`]-tors from `storage`.
///
/// There are no guarantees about position/order of any items left in the result [`Storage`], other
/// that they are all items (and only those items) that haven't been consumed (passed to `consume`).
// Not part of the contract/API: This starts removing items (the pivot) from `input` from its end,
// to avoid shuffling.
#[must_use]
pub fn qsort_idx<T, CONSUME>(
    input: Vec<T>,
    consume: &CONSUME,
    store_pair: StorePair<T>,
) -> InputStorePairCompletion<T>
where
    T: Ord,
    CONSUME: Fn(usize, T) -> bool,
{
    //consume_must_use_result::<T, _>(unsafe { mem::transmute(consume) });
    debug_assert_empty(&store_pair);
    let input_len = input.len();
    debug_assert_capacity(&store_pair, input_len);

    let mut next_out_idx = 0usize;
    let consume = make_consume_closure_must_use_result(consume);
    let (input, store_pair, completion) = part_idx(input, &consume, &mut next_out_idx, store_pair);
    debug_assert_eq!(
        next_out_idx + input.len() + store_pair[0].len() + store_pair[1].len(),
        input_len
    );
    (input, store_pair, completion)
}

/// - `next_out_seq_idx`: 0-based index of the next output item (increasing by one per each output
///    item but no correlation to its position in `input`).
#[must_use]
fn part_idx<T, CONSUME_MUST_USE_RESULT>(
    mut input: Vec<T>,
    consume: &CONSUME_MUST_USE_RESULT,
    next_out_idx: &mut usize,
    empty_store_pair: StorePair<T>,
) -> InputStorePairCompletion<T>
where
    T: Ord,
    CONSUME_MUST_USE_RESULT: Fn(usize, T) -> MustUse<bool>,
{
    debug_assert_empty(&empty_store_pair);
    debug_assert_capacity(&empty_store_pair, input.len());
    if input.is_empty() {
        return (input, empty_store_pair, true);
    }
    let pivot = input.pop().unwrap();
    let [mut lower_side, mut greater_equal_side] = empty_store_pair;

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
    let (input, lower_side, completion) = part_one_side(input, lower_side, consume, next_out_idx);
    // @TODO is `input` guaranteed to be empty?
    consume(*next_out_idx, pivot);
    *next_out_idx += 1;
    let (input, greater_equal_side, completion) =
        part_one_side(input, greater_equal_side, consume, next_out_idx);
    (input, [lower_side, greater_equal_side], completion)
}

#[must_use]
fn part_one_side<T, CONSUME_MUST_USE_RESULT>(
    empty_input: Vec<T>,
    mut side: Vec<T>,
    consume: &CONSUME_MUST_USE_RESULT,
    next_out_idx: &mut usize,
) -> InputStoreSingleCompletion<T>
where
    T: Ord,
    CONSUME_MUST_USE_RESULT: Fn(usize, T) -> MustUse<bool>,
{
    debug_assert!(empty_input.is_empty());
    // We reuse `input`: splitting it (by consuming it), then receiving it back in parts,
    // re-constructing it and returning (moving) it back.
    //
    // We also consume, receive & return `sub`.
    match side.len() {
        0 => (empty_input, side, true),
        1 => {
            consume(*next_out_idx, side.pop().unwrap());
            *next_out_idx += 1;
            (empty_input, side, true)
        }
        2 => {
            // Let's save splitting & reconstructing the Storage vectors: sort 2 items manually.
            let mut one = side.pop().unwrap();
            let mut two = side.pop().unwrap();
            if one > two {
                mem::swap(&mut one, &mut two);
            }
            consume(*next_out_idx, one);
            *next_out_idx += 1;
            consume(*next_out_idx, two);
            *next_out_idx += 1;
            (empty_input, side, true)
        }
        side_len => {
            let input_original_capacity = empty_input.capacity();

            // Set the capacity to any possible maximum, but not more - so that we catch any errors a.s.a.p.
            let store_pair = unsafe { split_vec(empty_input, side_len, side_len) };
            let (side, store_pair, completion) = part_idx(side, consume, next_out_idx, store_pair);
            debug_assert_eq!(side.len(), side_len); // ???

            let input = unsafe { join_vecs(store_pair) };

            debug_assert_eq!(input.capacity(), input_original_capacity);
            (input, side, completion)
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
unsafe fn split_vec<T>(input: Vec<T>, capacity_one: usize, capacity_two: usize) -> StorePair<T> {
    debug_assert!(capacity_one + capacity_two <= input.capacity());
    let (ptr, len, cap) = input.into_raw_parts();

    todo!()
}

/// Reconstruct a [`Vec`] from two split "subvectors". You must use this before you want to
/// [`Drop::drop`] it (them) automatically, or before you pass it outside this module (for re-use).
///
/// Only pass two adjacent [`Vec`]-tors returned from the same call to [`split_vec`].
unsafe fn join_vecs<T>(vecs: StorePair<T>) -> Vec<T> {
    todo!()
    //result
}
/*
pub fn qsort_len<T: Copy, S: Fn(usize, T)>(items: &mut [T], store: S, len: usize) {}

pub fn qsort_sub<T: Copy, S: Fn(usize, T), THRESHOLD: Fn(T, T) -> T>(
    items: &mut [T],
    store: S,
    threshold: THRESHOLD,
) {
}

pub fn qsort_with_acc<T: Copy, ACU, S: Fn(usize, T), ACCUMULATE: Fn(ACU, T) -> ACU>(
    items: &mut [T],
    store: S,
    accumulate: ACCUMULATE,
) {
}

fn part_and_collect<T: Copy, COLL, C: Fn(COLL, T) -> COLL, F: Fn(usize, T) -> bool>(
    items: &mut [T],
    f: F,
    next_result_item_idx: usize,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
}
*/
