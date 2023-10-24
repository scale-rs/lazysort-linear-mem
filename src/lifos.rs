use crate::calloc::{Allocator, Global, Vec, VecDeque};
use core::mem::{self, MaybeUninit};
use core::ptr;

#[cfg(test)]
mod lifos_tests;

/// See an example at
/// <https://doc.rust-lang.org/nightly/core/mem/union.MaybeUninit.html#initializing-an-array-element-by-element>
/// -> "(a) bunch of `MaybeUninit`s, which do not require initialization".

/// A contract on top of [`VecDeque`]. It (logically) keeps two LIFO (Last-In First-Out) queues,
/// growing in the opposite directions toward each other. (Similar to how stack & heap grow toward
/// each other in a single-threaded process/OS with no virtual memory, but with physical addressing
/// only):
/// ```
/// /*
/// /------------------------\
/// | FRONT          BACK    |
/// |    |           |       |
/// |    v           v       |
/// | abcd ->     <- 6543210 |
/// \------------------------/
///
/// (Assuming there has been at least 1 FRONT item BEFORE the first BACK item was put in. Otherwise
///  we temporarily transmute to VecDeque<MaybeUninit<T>>, put in a temporary uninitialized "front"
///  item, put in the actual back item, remove the temporary (uninitialized) front item, transmute
///  back to VecDeque<T>.)
/// */
/// ```
/// TODO report:
/// ```
/// // crossed in VS Code
/// /* not crossed */
/// ```
///
/// LIMITED so as NOT to expand/re-allocate. Keeping within the bounds is the responsibility of the
/// client - otherwise [`FixedDequeLifos::push_front()`] and [`FixedDequeLifos::push_front()`] will
/// panic (even in release)!
///
/// Minimum [`VecDeque`] capacity is 2 (even if you expect max. 1 item).
///
/// This *could* take [`VecDeque`] by mutable reference. But, it takes it owned (moved) instead -
/// because that suits [`CrossVecPairOrigin`].
#[derive(Debug)]
pub struct FixedDequeLifos<T, A: Allocator = Global> {
    vec_deque: VecDeque<T, A>,
    /// Front (left) side length.
    front: usize,
    /// Back (right) side length.
    back: usize,

    #[cfg(debug_assertions)]
    /// Used by checks for consistency & checks on push_front/push_back.
    original_capacity: usize,
}

// TODO
// - accept optional Alloc param.
/// This requires the backing [`VecDeque`] to be (initially) EMPTY.
impl<T, A: Allocator> From<VecDeque<T, A>> for FixedDequeLifos<T, A> {
    /// As per
    /// <https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html#impl-From%3CVec%3CT,+A%3E%3E-for-VecDeque%3CT,+A%3E>:
    /// "This conversion is guaranteed to run in O(1) time and to not re-allocate the Vec’s buffer
    fn from(mut vec_deque: VecDeque<T, A>) -> Self {
        debug_assert!(vec_deque.is_empty());
        // See also fn push_back(...).
        //
        // In general, the capacity does NOT need to be expected_ number_of_items+1. It is so only
        // if all you expect is one item: then the capacity must be at least 2 (which, in that
        // instance, happens to be expected_ number_of_items+1).
        //
        // But, if you expect more than 1 item, the capacity does NOT need to be higher than the
        // number of expected items - it may equal to that number.
        debug_assert!(vec_deque.capacity() >= 2, "In order not to re-allocate, the vec_deque must have capacity of at least 2 (even if you were expecting max. 1 item).");
        // Once .pop_front() or .pop_back() empty the VecDeque completely, according to their source
        // code (see linked from
        // <https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html#method.pop_front>
        // and
        // <https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html#method.pop_back>)
        // they do NOT ensure/reset the indices to 0 (to be contiguous). So WE ensure it here.
        vec_deque.make_contiguous();

        #[cfg(debug_assertions)]
        let original_capacity = vec_deque.capacity();

        let mut result = Self {
            vec_deque,
            front: 0,
            back: 0,
            #[cfg(debug_assertions)]
            original_capacity,
        };
        // TODO have this as a function, or clearer as a macro?
        result.debug_assert_consistent();
        result
    }
}
impl<T, A: Allocator> From<Vec<T, A>> for FixedDequeLifos<T, A> {
    /// As per
    /// <https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html#impl-From%3CVec%3CT,+A%3E%3E-for-VecDeque%3CT,+A%3E>:
    /// "This conversion is guaranteed to run in O(1) time and to not re-allocate the Vec’s buffer
    fn from(v: Vec<T, A>) -> Self {
        let vec_deque: VecDeque<T, A> = v.into();
        vec_deque.into()
    }
}

impl<T, A: Allocator> FixedDequeLifos<T, A> {
    pub fn new_from_empty(vec_deque: VecDeque<T, A>) -> Self {
        vec_deque.into()
    }

    /// Consume this instance, and return the underlying [`VecDeque`]. Sufficient for use by
    /// [`CrossVecPairGuard`], which (instead of [`FixedDequeLifos::front`] and
    /// [`FixedDequeLifos::back`]) uses [`VecDeque::as_mut_slices()`] to retrieve both the front &
    /// back data section. (And [`FixedDequeLifos`] maintains integrity, so that
    /// [`FixedDequeLifos::front`] & [`FixedDequeLifos::back`] and the underlying [`VecDeque`] are
    /// always in sync.)
    ///
    /// Intentionally NOT called `into()`, so that if we (ever) add implementation(s) of [`Into`],
    /// the function names would be clear.
    ///
    /// TODO: Should we implement `impl<T> Into<()> for FixedDequeLifos<T>`? Because even if we do,
    /// if we then call .into(), we HAVE TO specify the result type anyway.
    pub fn into_vec_deque(mut self) -> VecDeque<T, A> {
        self.debug_assert_consistent();
        self.vec_deque
    }

    pub fn push_front(&mut self, value: T) {
        self.debug_assert_consistent();
        self.assert_reserve_for_one();

        // We can always push to front, regardless of whether there is any back item or not. This
        // will not upset the back part (slice) positioning. (And, if there were no item yet at all
        // - neither at the front, nor at the back, then this will enable easier push to the back
        // from now on.)
        self.vec_deque.push_front(value);
        self.front += 1;

        self.debug_assert_consistent();
    }

    pub fn push_back(&mut self, value: T) {
        self.debug_assert_consistent();

        if !self.vec_deque.is_empty() {
            self.assert_reserve_for_one();
            self.vec_deque.push_back(value);
        } else {
            self.assert_total_capacity_for_two();

            unsafe {
                let vec_deque = ptr::read(&self.vec_deque as *const VecDeque<T, A>);
                let mut vec_deque =
                    mem::transmute::<VecDeque<T, A>, VecDeque<MaybeUninit<T>, A>>(vec_deque);

                vec_deque.push_front(MaybeUninit::uninit());
                vec_deque.push_back(MaybeUninit::new(value));
                let popped = vec_deque.pop_front();
                debug_assert!(popped.is_some());

                let vec_deque = mem::transmute::<_, VecDeque<T, A>>(vec_deque);
                ptr::write(&mut self.vec_deque as *mut VecDeque<T, A>, vec_deque);
            }
        }
        self.back += 1;

        self.debug_assert_consistent();
    }

    pub fn front(&self) -> usize {
        self.front
    }
    pub fn back(&self) -> usize {
        self.back
    }

    #[inline(always)]
    fn debug_assert_consistent(&self) {
        #[cfg(debug_assertions)]
        debug_assert_eq!(self.original_capacity, self.vec_deque.capacity());
        debug_assert_eq!(self.front + self.back, self.vec_deque.len());
        debug_assert!({
            let (front, back) = self.vec_deque.as_slices();
            debug_assert_eq!(self.front, front.len());
            debug_assert_eq!(self.back, back.len());
            true
        });
    }

    /// NON-debug assert: run in RELEASE, too. Otherwise client's mistakes could lead to undefined
    /// behavior.
    #[inline(always)]
    fn assert_reserve_for_one(&self) {
        assert!(self.vec_deque.len() < self.vec_deque.capacity());
    }

    /// NON-debug assert: run in RELEASE, too. Call only on empty: specialized for use by
    /// `push_back(...)`.
    #[inline(always)]
    fn assert_total_capacity_for_two(&self) {
        debug_assert!(
            self.vec_deque.is_empty(),
            "This can be called only when vec_deque is empty. But it has {} item(s) instead!",
            self.vec_deque.len()
        );
        assert!(self.vec_deque.capacity() >= 2);
    }
}
