use crate::calloc::calloc_vec::{Vec, VecDeque};
use crate::calloc::{Allocator, Global};
use crate::store::lifos::Lifos;
use core::mem::{self, MaybeUninit};
use core::ptr;

#[cfg(test)]
mod lifos_vec_tests;

/// A contract on top of [`VecDeque`]. It (logically) keeps two LIFO (Last-In First-Out) queues,
/// growing in the opposite directions toward each other. (Similar to how stack & heap grow toward
/// each other in a single-threaded process/OS with no virtual memory, but with physical addressing
/// only):
/// ```
/// /*
/// /------------------------\
/// | LEFT           RIGHT   |
/// | (back)         (front) |
/// |    |           |       |
/// |    v           v       |
/// | abcd ->     <- 6543210 |
/// \------------------------/
///
/// (Assuming there has been at least 1 LEFT item (pushed "back") BEFORE the first RIGHT item was
///  put in (pushed to "front"). Otherwise we temporarily transmute to VecDeque<MaybeUninit<T>>,
///  put in a temporary uninitialized LEFT ("back") item, put in the actual RIGHT (front) item,
///  remove the temporary (uninitialized) LEFT (back) item, transmute back to VecDeque<T>.)
/// */
/// ```
/// See an example at
/// <https://doc.rust-lang.org/nightly/core/mem/union.MaybeUninit.html#initializing-an-array-element-by-element>
/// -> "(a) bunch of `MaybeUninit`s, which do not require initialization".
///
///
/// TODO report VS Code doc comment formatting:
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
/// because that suits [`crate::cross::CrossVecPairGuard`].
///
/// Based on source code of [`alloc::collections::VecDeque`] (for non-empty buffer, and for
/// non-zero-sized item types):
/// ```
/// /*
/// vec_deque.front() == vec_deque.get(0) == "head" item
///
/// vec_deque.back() == vec_deque.get( vec_deque.len()-1 ) == "back" item
///
/// contiguous: head+len < buf.cap
/// /---------------------------------------------\
/// | buffer direction -->                        |
/// |                                             |
/// | 0                  head=21 (logical idx 0)  |
/// | |                    |                      |
/// | v                    v                      |
/// | ----------------------                      |
/// | |    head=21         |                      |
/// | |                  get(0)  get(len-1)       |
/// | |                    |       |              |
/// | |                  front   back  head+len   |
/// | |                    |       |  /           |
/// | |                  head      | /            |
/// | |                    |       ||             |
/// | v                    v       vv             |
/// | _____________________abcdefghi____________  |
/// | ^      (unused)      len: 9     (unused) ^  |
/// | |                                        |  |
/// | buf[0]                      buf[buf.cap-1]  |
/// | |                                        |  |
/// | ___________________________________________ |
/// |          buf.cap = 42                       |
/// \---------------------------------------------/
///
/// non-contiguous: head+len >= buf.len, wrap around:
/// /---------------------------------------------\
/// | back  head+len-buf.cap  front               |
/// |  |    21+26-42 = 5       /                  |
/// |  \      /               head=21 (logical 0) |
/// |   \    /               /                    |
/// |    \  /               /                     |
/// |     ||               |                      |
/// |     vv               v                      |
/// | vwxyz________________abcdefghijklmnopqrstu  |
/// | ^    ^   (unused)    len: 26             ^  |
/// | |    |                                   |  |
/// | 0    5                      buf[buf.cap-1]  |
/// \---------------------------------------------/
///
/// <--- vec_deque.push_front(item) inserts before "head" (front)
///      vec_deque.push_back(item)  appends at the end (back) --->
///
/// */
/// ```
#[derive(Debug)]
pub struct FixedDequeLifos<T, A: Allocator = Global> {
    vec_deque: VecDeque<T, A>,
    /// Left ("back") side length.
    left: usize,
    /// Right ("front") side length.
    right: usize,

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
        // See also fn push_right(...).
        //
        // In general, the capacity does NOT need to be expected_number_of_items+1. It is so only if
        // all you expect is one item: then the capacity must be at least 2 (which, in that
        // instance, happens to be expected_number_of_items+1). That's because we need ability to
        // allocate an extra temporary item on the LEFT ("back") if the VERY FIRST push is on the
        // RIGHT ("front").
        //
        // But, if you expect more than 1 item, the capacity does NOT need to be higher than the
        // expected_number_of_items - it may equal to that number.
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

        let result = Self {
            vec_deque,
            left: 0,
            right: 0,
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
    /// [`CrossVecPairGuard`], which (instead of [`FixedDequeLifos::left`] and
    /// [`FixedDequeLifos::right`]) uses [`VecDeque::as_mut_slices()`] to retrieve both the left &
    /// right data section. (And [`FixedDequeLifos`] maintains integrity, so that
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

    #[inline(always)]
    fn debug_assert_consistent(&self) {
        #[cfg(debug_assertions)]
        debug_assert_eq!(self.original_capacity, self.vec_deque.capacity());
        debug_assert_eq!(self.left + self.right, self.vec_deque.len());
        debug_assert!({
            let (back, front) = self.vec_deque.as_slices();
            debug_assert_eq!(back.len(), self.left);
            debug_assert_eq!(front.len(), self.right);
            true
        });
    }

    /// NON-debug assert: run in RELEASE, too. Otherwise client's mistakes could lead to undefined
    /// behavior.
    #[inline(always)]
    fn assert_reserve_for_one(&self) {
        assert!(self.vec_deque.len() < self.vec_deque.capacity());
    }

    /// NON-debug assert: running in RELEASE, too. Call only on empty: specialized for use by
    /// `push_right(...)`.
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

impl<T, A: Allocator> Lifos<T> for FixedDequeLifos<T, A> {
    fn has_to_push_left_first() -> bool {
        true
    }

    fn push_left(&mut self, value: T) {
        self.debug_assert_consistent();
        self.assert_reserve_for_one();

        // We can always push to LEFT (VecDeque back), regardless of whether there is any RIGHT
        // (front) item or not. This will not upset the RIGHT (front) slice. (And, if there were no
        // items yet at all - neither on the LEFT (VecDeque back), nor on the RIGHT (VecDeque
        // front), then this will enable easier push to the RIGHT (VecDeque front) from now on.
        self.vec_deque.push_back(value);
        self.left += 1;

        self.debug_assert_consistent();
    }

    fn push_right(&mut self, value: T) {
        self.debug_assert_consistent();

        if !self.vec_deque.is_empty() {
            self.assert_reserve_for_one();
            self.vec_deque.push_front(value);
        } else {
            self.assert_total_capacity_for_two();

            unsafe {
                // The following failed to compile with our crate's feature
                // `_internal_use_allocator_api` (on `nightly`)
                //let vec_deque = ptr::read(&self.vec_deque as *const VecDeque<T, A>);
                //let mut vec_deque =
                //    mem::transmute::<VecDeque<T, A>, VecDeque<MaybeUninit<T>, A>>(vec_deque);

                // TODO is this sound?
                let mut vec_deque =
                    ptr::read(&self.vec_deque as *const _ as *const VecDeque<MaybeUninit<T>, A>);

                vec_deque.push_back(MaybeUninit::uninit());
                vec_deque.push_front(MaybeUninit::new(value));
                let popped = vec_deque.pop_back();
                debug_assert!(popped.is_some());

                // The following caused an error again:
                // let vec_deque = mem::transmute::<_, VecDeque<T, A>>(vec_deque);
                // ptr::write(&mut self.vec_deque as *mut VecDeque<T, A>, vec_deque);

                // TODO the below active (uncommented) code sound? If not, how about the following
                // (commented) code?
                // let tmp_vec_deque = vec_deque;
                // let vec_deque = ptr::read(&tmp_vec_deque as *const _ as *const MaybeUninit<VecDeque<T, A>>);
                // mem::forget(tmp_vec_deque);

                ptr::write(
                    &mut self.vec_deque as *mut _ as *mut VecDeque<MaybeUninit<T>, A>,
                    vec_deque,
                );
            }
        }
        self.right += 1;

        self.debug_assert_consistent();
    }

    fn right(&self) -> usize {
        self.right
    }
    fn left(&self) -> usize {
        self.left
    }
}
