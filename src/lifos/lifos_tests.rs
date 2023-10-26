use crate::calloc::VecDeque;
use crate::lifos::FixedDequeLifos;
use std::time::{SystemTime, UNIX_EPOCH};

extern crate std;

#[test]
fn left_right_left() {
    let mut lifos = FixedDequeLifos::<u8>::new_from_empty(VecDeque::<u8>::with_capacity(3));
    lifos.push_left(1);
    lifos.push_right(2);
    lifos.push_left(3);
}
#[test]
fn right_left_right() {
    let mut lifos = FixedDequeLifos::<u8>::new_from_empty(VecDeque::<u8>::with_capacity(7));
    lifos.push_right(1);
    lifos.push_left(2);
    lifos.push_left(1);
}

// ------------
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

const MIN_VEC_DEQUE_CAPACITY: u32 = 2;
const MAX_VEC_DEQUE_CAPACITY: u32 = 65535;

fn empty_vec_deque_puts_back_item_to_front(capacity: usize) {
    assert!(capacity >= MIN_VEC_DEQUE_CAPACITY as usize);
    let mut vec_deque = VecDeque::<bool>::with_capacity(capacity);

    vec_deque.push_back(true);
    let (front, back) = vec_deque.as_slices();

    assert_eq!(front.len(), 1);
    assert!(back.is_empty());
}

fn single_item_vec_deque_rotate_left_does_not_circular(capacity: usize) {
    assert!(capacity >= MIN_VEC_DEQUE_CAPACITY as usize);
    let mut vec_deque = VecDeque::<bool>::with_capacity(capacity);

    vec_deque.push_back(true);
    vec_deque.rotate_left(1);
    let (front, back) = vec_deque.as_slices();

    assert_eq!(front.len(), 1);
    assert!(back.is_empty());
}

/// If this ever fails, it means we don't need to have our MaybeUninit workaround. Then
/// - feel free to disable this test, or even better: reverse it
/// - undo the MaybeUninit part in [crate::lifos::FixedDequeLifos]
/// - if your Rust & platform are mainstream or upcoming, please report the details, so we fix both above for
///   such a Rust/platform combination.
///
/// If this test succeeds, it demonstrates the problem situation which caused us to (temporarily)
/// use MaybeUninit in in [crate::lifos::FixedDequeLifos] until the first .push_front(..).
#[test]
fn empty_vec_deque_puts_back_item_to_front_for_capacities() {
    empty_vec_deque_puts_back_item_to_front(MIN_VEC_DEQUE_CAPACITY as usize);

    let capacity = rnd_u32(MIN_VEC_DEQUE_CAPACITY, MAX_VEC_DEQUE_CAPACITY) as usize;
    empty_vec_deque_puts_back_item_to_front(capacity);

    empty_vec_deque_puts_back_item_to_front(MAX_VEC_DEQUE_CAPACITY as usize);
}

/// If this ever fails, it means we don't need to have our MaybeUninit workaround.
///
/// If this test succeeds, it demonstrates: If we're putting in the first item to a [`VecDeque`],
/// and putting it to __back__, even if we then `vec_deque.rotate_left(1)`, it will not move that
/// (single) item to the right side of the [`VecDeque`].
#[test]
fn single_item_vec_deque_rotate_left_does_not_circular_for_capacities() {
    single_item_vec_deque_rotate_left_does_not_circular(MIN_VEC_DEQUE_CAPACITY as usize);

    let capacity = rnd_u32(MIN_VEC_DEQUE_CAPACITY, MAX_VEC_DEQUE_CAPACITY) as usize;
    single_item_vec_deque_rotate_left_does_not_circular(capacity);

    single_item_vec_deque_rotate_left_does_not_circular(MAX_VEC_DEQUE_CAPACITY as usize);
}
