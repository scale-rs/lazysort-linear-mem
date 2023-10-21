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
pub type Storage<T> = [Vec<T>; 2];
pub type InputAndStorage<T> = (Vec<T>, Storage<T>);

fn debug_assert_all_empty<T>(storage: &Storage<T>) {
    debug_assert!(storage[0].is_empty());
    debug_assert!(storage[1].is_empty());
}

/// Assert that all [`Vec`] in `storage` have sufficient capacity (equal to, or greater than, `capacity`).
fn debug_assert_all_capacity<T>(storage: &Storage<T>, capacity: usize) {
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
    input: Vec<T>,
    consume: &CONSUME,
    storage: Storage<T>,
) -> InputAndStorage<T>
where
    T: Ord,
    CONSUME: Fn(usize, T) -> bool,
{
    debug_assert_all_empty(&storage);
    debug_assert_all_capacity(&storage, input.len());

    let mut next_out_seq_idx = 0usize;
    part(input, consume, &mut next_out_seq_idx, storage)
}

/// - `next_out_seq_idx`: 0-based index of the next output item (increasing by one per each output
///    item but no correlation to its position in `input`).
#[must_use]
fn part<T, CONSUME>(
    mut input: Vec<T>,
    consume: &CONSUME,
    next_out_seq_idx: &mut usize,
    storage: Storage<T>,
) -> InputAndStorage<T>
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
    // We reuse `input` and `lower`, consuming them, then returning (moving) them back.
    let (input, lower) = sub_part(input, lower, consume, next_out_seq_idx);
    /*let (input, lower) = match lower.len() {
        0 => (input, lower),
        1 => {
            consume(*next_out_seq_idx, lower.pop().unwrap());
            *next_out_seq_idx += 1;
            (input, lower)
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
            (input, lower)
        }
        lower_len => {
            let (storage, original_capacity) = unsafe { split_vec(input, lower_len, lower_len) };
            let (lower, storage) = part(lower, consume, next_out_seq_idx, storage);
            (unsafe { join_vecs(storage, original_capacity) }, lower)
        }
    };*/
    consume(*next_out_seq_idx, pivot);
    *next_out_seq_idx += 1;
    let (input, greater_equal) = sub_part(input, greater_equal, consume, next_out_seq_idx);
    (input, [lower, greater_equal])
}

#[must_use]
fn sub_part<T, CONSUME>(
    mut input: Vec<T>,
    mut sub: Vec<T>,
    consume: &CONSUME,
    next_out_seq_idx: &mut usize,
) -> (Vec<T>, Vec<T>)
where
    T: Ord,
    CONSUME: Fn(usize, T) -> bool,
{
    debug_assert!(input.is_empty());
    // We reuse `input`: splitting it (by consuming it), then receiving it back in parts,
    // re-constructing it and returning (moving) it back.
    //
    // We also consume, receive & return `sub`.
    match sub.len() {
        0 => (input, sub),
        1 => {
            consume(*next_out_seq_idx, sub.pop().unwrap());
            *next_out_seq_idx += 1;
            (input, sub)
        }
        2 => {
            // Let's save splitting & reconstructing the Storage vectors: sort 2 items manually.
            let mut one = sub.pop().unwrap();
            let mut two = sub.pop().unwrap();
            if one > two {
                mem::swap(&mut one, &mut two);
            }
            consume(*next_out_seq_idx, one);
            *next_out_seq_idx += 1;
            consume(*next_out_seq_idx, two);
            *next_out_seq_idx += 1;
            (input, sub)
        }
        sub_len => {
            let (storage, original_capacity) = unsafe { split_vec(input, sub_len, sub_len) };
            let (sub, storage) = part(sub, consume, next_out_seq_idx, storage);
            debug_assert_eq!(sub.len(), sub_len);
            (unsafe { join_vecs(storage, original_capacity) }, sub)
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
unsafe fn split_vec<T>(
    input: Vec<T>,
    capacity_one: usize,
    capacity_two: usize,
) -> (Storage<T>, usize) {
    debug_assert!(capacity_one + capacity_two <= input.capacity());
    loop {}
}

/// Reconstruct a [`Vec`] from two split "subvectors". You must use this before you want to
/// [`Drop::drop`] it (them) automatically, or before you pass it outside this module (for re-use).
///
/// Only pass two adjacent [`Vec`]-tors returned from the same call to [`split_vec`].
unsafe fn join_vecs<T>(vecs: Storage<T>, original_capacity: usize) -> Vec<T> {
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
