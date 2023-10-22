use alloc::collections::VecDeque;

/// A contract on top of [`VecDeque`]. It (logically) keeps two LIFOs (Last-In First-Out queues),
/// growing in the opposite directions toward each other. Similar to how stack & heap grow toward
/// each other (in a single-threaded process/OS with no virtual memory, but with physical addressing
/// only):
/// ```
/// /*
/// /----------------------\
/// | abcd ->     <- edcba |
/// \----------------------/
/// */
/// ```
/// TODO report:
/// ````
/// // crossed in VS Code
/// /* not crossed */
/// ```
///
/// LIMITED so as NOT to expand/re-allocate. It's the responsibility of the client!
///
/// This *could* take [`VecDeque`] by mutable reference. But, it takes it owned (moved) instead -
/// because that suits [`CrossVecPairOrigin`].
pub struct FixedDequeLifos<T> {
    vec_deque: VecDeque<T>,
    /// Front (left) side length.
    front: usize,
    /// Back (right) side length.
    back: usize,
    #[cfg(debug_assertions)]
    /// Used by checks for consistency & checks on push_front/push_back.
    original_capacity: usize,
}
impl<T> From<VecDeque<T>> for FixedDequeLifos<T> {
    /// As per
    /// <https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html#impl-From%3CVec%3CT,+A%3E%3E-for-VecDeque%3CT,+A%3E>:
    /// "This conversion is guaranteed to run in O(1) time and to not re-allocate the Vec’s buffer
    /// or allocate any additional memory."
    fn from(mut vec_deque: VecDeque<T>) -> Self {
        debug_assert!(vec_deque.is_empty());
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
impl<T> FixedDequeLifos<T> {
    pub fn new_from_empty(vec_deque: VecDeque<T>) -> Self {
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
    pub fn into_vec_deque(self) -> VecDeque<T> {
        self.debug_assert_consistent();
        self.vec_deque
    }

    pub fn push_front(&mut self, value: T) {
        self.debug_assert_capacity_for_one();
        self.vec_deque.push_front(value);
        self.front += 1;
        self.debug_assert_consistent();
    }
    pub fn push_back(&mut self, value: T) {
        self.debug_assert_capacity_for_one();
        debug_assert!(self.vec_deque.len() < self.vec_deque.capacity());
        self.vec_deque.push_back(value);
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
    #[inline(always)]
    fn debug_assert_capacity_for_one(&self) {
        debug_assert!(self.vec_deque.len() < self.vec_deque.capacity());
    }
}
