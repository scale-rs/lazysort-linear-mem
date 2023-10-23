use crate::calloc::VecDeque;

extern crate std;

use core::char::MAX;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RND: u32 = 1000_000_000;
// Thanks to https://blog.orhun.dev/zero-deps-random-in-rust/
/// Return a (semi)random number, between 0 (inclusive) up to `max` (exclusive). `max-min` must be
/// less than, or equal to, 1 billion (10 to power of 9).
///
/// For high `max-min` (comparable to 1 billion) this is biased towards the lower part of the range.
/// (Of course, `max` must be higher than `min`.)
///
/// NOT crypto-secure - for testing/controlled/isolated runs only.
fn rnd_u32(min: u32, max: u32) -> u32 {
    let range_width = max - min;
    assert!(range_width > 0);
    assert!(range_width <= MAX_RND);
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => min + (duration.subsec_nanos() % range_width),
        Err(err) => panic!("Failed to get system time nanoseconds: {}", err),
    }
}

fn empty_vec_deque_puts_back_item_to_back_for_capacity(capacity: usize) {
    assert!(capacity >= 2);
    let mut vec_deque = VecDeque::<bool>::with_capacity(capacity);

    vec_deque.push_back(true);
    let (front, back) = vec_deque.as_slices();

    assert_eq!(front.len(), 1);
    assert!(back.is_empty());
}

const MAX_VEC_DEQUE_CAPACITY: u32 = 65535;

/// If this ever fails, it means we don't need to have our MaybeUninit workaround. Then
/// - feel free to disable this test, or even better: reverse it
/// - undo the MaybeUninit part in [crate::lifos::FixedDequeLifos]
/// - if your Rust & platform are mainstream or upcoming, please report the details, so we fix both above for
///   such a Rust/platform combination.
///
/// If this test succeeds, it demonstrates the problem situation which caused us to (temporarily)
/// use MaybeUninit in in [crate::lifos::FixedDequeLifos] until the first .push_front(..).
#[test]
fn empty_vec_deque_puts_back_item_to_back() {
    empty_vec_deque_puts_back_item_to_back_for_capacity(2);

    let capacity = rnd_u32(2, MAX_VEC_DEQUE_CAPACITY) as usize;
    empty_vec_deque_puts_back_item_to_back_for_capacity(capacity);

    empty_vec_deque_puts_back_item_to_back_for_capacity(MAX_VEC_DEQUE_CAPACITY as usize);
}
