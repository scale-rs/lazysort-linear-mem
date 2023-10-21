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
pub type Storage<T, const N: usize> = [Vec<T>; N];

fn debug_assert_all_empty<T, const N: usize>(storage: &Storage<T, N>) {
    debug_assert!({
        for i in 0..N {
            debug_assert!(storage[i].is_empty());
        }
        true
    });
}

/// Assert that all [`Vec`] in `storage` have sufficient capacity (equal to, or greater than, `capacity`).
fn debug_assert_all_capacity<T>(storage: &Storage<T, 2>, capacity: usize) {
    debug_assert!(storage[0].capacity() >= capacity);
    debug_assert!(storage[1].capacity() >= capacity);
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
pub fn qsort<T, CONSUME>(
    mut input: Vec<T>,
    consume: CONSUME,
    storage: Storage<T, 2>,
) -> Storage<T, 3>
where
    T: Ord,
    CONSUME: Fn(usize, T) -> bool,
{
    debug_assert_all_empty(&storage);
    debug_assert_all_capacity(&storage, input.len());

    loop {}
}

/// - `next_out_seq_idx`: 0-based index of the next output item (increasing by one per each output
///    item but no correlation to its position in `input`).
#[must_use]
fn part<T, CONSUME>(
    mut input: Vec<T>,
    consume: &CONSUME,
    next_out_seq_idx: &mut usize,
    storage: Storage<T, 2>,
) -> (Vec<T>, Storage<T, 2>)
where
    T: Ord,
    CONSUME: Fn(usize, T) -> bool,
{
    debug_assert_all_empty(&storage);
    debug_assert_all_capacity(&storage, input.len());
    if input.is_empty() {
        return (input, storage);
    }
    let pivot = input.pop().unwrap();
    let [mut lower, mut greater_equal] = storage;

    while !input.is_empty() {
        let value = input.pop().unwrap();
        if value < pivot {
            lower.push(value);
        } else {
            greater_equal.push(value);
        }
    }
    // We reuse `input`, consuming it, then returning (moving) it back.
    let input = match lower.len() {
        0 => input,
        1 => {
            consume(*next_out_seq_idx, lower.pop().unwrap());
            *next_out_seq_idx += 1;
            input
        }
        2 => {
            // Let's save splitting & reconstructing the Storage vectors: sort 2 items manually.
            let mut one = lower.pop().unwrap();
            let mut two = lower.pop().unwrap();
            if one > two {
                mem::swap(&mut one, &mut two);
            }
            consume(*next_out_seq_idx, one);
            *next_out_seq_idx += 1;
            consume(*next_out_seq_idx, two);
            *next_out_seq_idx += 1;
            input
        }
        lower_len => {
            let (storage, original_capacity) =
                unsafe { split_vec(input, lower_len, lower_len) };
            let (lower, storage) = part(lower, consume, next_out_seq_idx, storage);
            unsafe { join_vecs(storage, original_capacity) }
        }
    };
    consume(*next_out_seq_idx, pivot);
    *next_out_seq_idx += 1;
    loop {}
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
unsafe fn split_vec<T>(
    input: Vec<T>,
    capacity_one: usize,
    capacity_two: usize,
) -> (Storage<T, 2>, usize) {
    debug_assert!(capacity_one + capacity_two <= input.capacity());
    loop {}
}

/// Reconstruct a [`Vec`] from two split "subvectors". You must use this before you want to
/// [`Drop::drop`] it (them) automatically, or before you pass it outside this module (for re-use).
///
/// Only pass two adjacent [`Vec`]-tors returned from the same call to [`split_vec`].
unsafe fn join_vecs<T>(vecs: Storage<T, 2>, original_capacity: usize) -> Vec<T> {
    loop {}
}

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
