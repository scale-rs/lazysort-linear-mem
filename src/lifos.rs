use crate::calloc::{Allocator, Global, Vec, VecDeque};
use core::mem::MaybeUninit;
use core::ptr;

// - TODO no-alloc-friendly "SliceDeque" struct
// - TODO when Storage is backed by an array, make the array size a const generic
// - TODO a trait and an adapter for VecDeque

use core::num::{NonZeroU16, NonZeroU8, NonZeroUsize};
/// Non-recursive implementation
///
/// Trait used for indexing of tree-like nodes within Vec/VecDeque-like linear storage.
///
/// It leverages optimization with [`Option`] for [`NonZeroUsize`], and for some of [`NonZeroU8`],
/// [`NonZeroU16`]... types. [`Option`] for those types doesn't take any extra space.
///
/// However, out of `NonZeroUxyz` types, it's possible to implement it for [`NonZeroUsize`] and only
/// for [`NonZeroU8`], [`NonZeroU16`]... types that are smaller or same width as [`NonZeroUsize`]
/// (on a particular platform/target).
///
/// TODO
///
/// Trait replacing Vec/VecDeque for input + storage
/// - TODO Elements for storage: Generic over u8/u16/u32/u64/usize INDEX type
/// - TODO implementation struct: 1 Vec/SliceVec <-> VecDeque/SliceDeque for both custom items (ones
///   being sorted, of type T), and for and INDEX and related metadata along in a struct.
///   Disadvantage: When used as Vec/SliceVec (for read-only "input", rather than for mutable 2-lifo
///   "storage"), INDEX+metadata slots are unused, hence unused memory throughout the Vec/SliceVec.
/// - TODO implementation with 2 structs: 1 Vec/SliceVec + 1 VecDeque/SliceDeque.
trait Index: Eq + Ord + Sized {
    fn min_index_usize() -> usize {
        Self::min_index().to_usize()
    }
    fn min_index() -> Self;

    /// See [`max_index()`].
    fn max_index_usize() -> usize {
        Self::max_index().to_usize()
    }
    /// Implementation of this function for [`NonZeroUsize`], and for [`NonZeroUxyz`] type whose
    /// width is the same to that of [`NonZeroUsize`]), is an exception. For any "smaller" types ([`NonZeroU8`]...) it
    /// can return the max. value of that type. But for [`NonZeroUsize`], and for [`NonZeroUxyz`]
    /// type whose width is the same to that of [`NonZeroUsize`]), this can return the maximum value
    /// of that type minus 1. (Because an array/slice max. length is [`usize::MAX`], so any
    /// index has to be smaller.)
    fn max_index() -> Self;
    fn max_indexable_len() -> usize;

    /// Length (range width) indexable by this type, given a physical length.
    fn indexable_len(physical_len: usize) -> usize {
        assert!(physical_len >= Self::min_index_usize());
        physical_len - Self::min_index_usize()
    }
    /// - u8/u16...usize: physical_len==3: `012` -> max. exl. 3
    /// - NonZeroU8...  : physical_len==3: ` 12` -> max. exl. 3
    /// - When we index by [`NonZeroU8`] etc, we do NOT subtract 1. We use the index as-is. Yes, we
    ///   do "waste" the item at index 0.
    fn max_index_excl_usize(physical_len: usize) -> usize {
        panic!("not needed?")
    }
    /// - u8/u16...usize: physical_len==3: `012` -> max. incl. 2
    /// - NonZeroU8...  : physical_len==3: ` 12` -> max. incl. 2
    fn max_index_incl_usize(physical_len: usize) -> usize {
        panic!("not needed?")
    }

    fn from_usize(index: usize) -> Self;
    fn to_usize(&self) -> usize;
}

/// Working around [`Option::unwrap()`] not being a const function (yet).
const fn unwrap_option<T: Copy>(opt: Option<T>) -> T {
    match opt {
        Some(t) => t,
        None => panic!(),
    }
}

// ---- Constants for various implementations of [`Index`] first See those implementations first.

/// Different to most `Uxyz_MAX_INDEX_USIZE` (other than `Uxyz_MAX_INDEX_USIZE` for `uxyz` primitive
/// type with byte width same as that of `usize`).
///
/// It CANNOT be [`usize::MAX`] itself, because that's the maximum possible/indexable length of any
/// array/slice. If there existed an array/slice with a maximum index being `usize::MAX`, the length
/// of such an array/slice would be more than `usize::MAX`, which cannot exist in Rust!
const USIZE_MAX_INDEX_USIZE: usize = usize::MAX - 1;
const USIZE_MAX_INDEX: usize = USIZE_MAX_INDEX_USIZE;

/// Different to most `Uxyz_MAX_INDEXABLE_LEN`` (other than `Uxyz_MAX_INDEXABLE_LEN` for `uxyz`
/// primitive type with byte width same as that of `usize`).
///
/// `0..=USIZE_MAX_INDEX_USIZE`
const USIZE_MAX_INDEXABLE_LEN: usize = USIZE_MAX_INDEX_USIZE + 1;
const _: () = {
    if USIZE_MAX_INDEXABLE_LEN != usize::MAX {
        panic!();
    }
};
// --

/// Different to most `NON_ZERO_Uxyz_MAX_INDEX_USIZE` (other than `NON_ZERO_Uxyz_MAX_INDEX_USIZE`
/// for `NonZeroUxyz` type with byte width same as that of `usize`). Why is it
/// `NonZeroUsize::MAX.to_usize() - 1` and not just `NonZeroUsize::MAX.to_usize()`? See
/// [`USIZE_MAX_INDEX_USIZE`].
const NON_ZERO_USIZE_MAX_INDEX_USIZE: usize = NonZeroUsize::MAX.get() - 1;
const NON_ZERO_USIZE_MAX_INDEX: NonZeroUsize =
    unwrap_option(NonZeroUsize::new(NON_ZERO_USIZE_MAX_INDEX_USIZE));
/// Different to most `NON_ZERO_Uxyz_MAX_INDEXABLE_LEN` (other than
/// `NON_ZERO_Uxyz_MAX_INDEXABLE_LEN` for `NonZeroUxyz` type with byte width same as that of
/// `usize`).
///
/// 1..=NON_ZERO_USIZE_MAX_INDEX_USIZE
const NON_ZERO_USIZE_MAX_INDEXABLE_LEN: usize = NON_ZERO_USIZE_MAX_INDEX_USIZE;
const _: () = {
    if NON_ZERO_USIZE_MAX_INDEXABLE_LEN != usize::MAX - 1 {
        panic!()
    }
};
// --

const U8_MAX_INDEX_USIZE: usize = u8::MAX as usize;
const U8_MAX_INDEX: u8 = u8::MAX;
/// `0..=U8_MAX_INDEX_USIZE` == 0..=u8::MAX == 0..=255 == 256 slots
const U8_MAX_INDEXABLE_LEN: usize = U8_MAX_INDEX_USIZE + 1;
const _: () = {
    if U8_MAX_INDEXABLE_LEN != 256 {
        panic!()
    }
};
// --

const NON_ZERO_U8_MAX_INDEX_USIZE: usize = NonZeroU8::MAX.get() as usize;
const NON_ZERO_U8_MAX_INDEX: NonZeroU8 = NonZeroU8::MAX;
/// `1..=NON_ZERO_U8_MAX_INDEX_USIZE` == 1..=u8::MAX == 1..255 == 255 slots
const NON_ZERO_U8_MAX_INDEXABLE_LEN: usize = NON_ZERO_U8_MAX_INDEX_USIZE;
const _: () = {
    if NON_ZERO_U8_MAX_INDEXABLE_LEN != 255 {
        panic!()
    }
};
// --

impl Index for usize {
    fn min_index_usize() -> usize {
        0
    }
    fn min_index() -> Self {
        0
    }

    fn max_index_usize() -> usize {
        USIZE_MAX_INDEX_USIZE
    }
    fn max_index() -> Self {
        USIZE_MAX_INDEX
    }

    fn max_indexable_len() -> usize {
        USIZE_MAX_INDEXABLE_LEN
    }
    fn from_usize(index: usize) -> Self {
        index
    }
    fn to_usize(&self) -> usize {
        *self
    }
}

impl Index for NonZeroUsize {
    fn min_index_usize() -> usize {
        NonZeroUsize::MIN.get()
    }
    fn min_index() -> Self {
        NonZeroUsize::MIN
    }

    fn max_index_usize() -> usize {
        NON_ZERO_USIZE_MAX_INDEX_USIZE
    }
    fn max_index() -> Self {
        NON_ZERO_USIZE_MAX_INDEX
    }

    fn max_indexable_len() -> usize {
        NON_ZERO_USIZE_MAX_INDEXABLE_LEN
    }
    fn from_usize(index: usize) -> Self {
        NonZeroUsize::new(index).unwrap()
    }
    fn to_usize(&self) -> usize {
        self.get()
    }
}

impl Index for u8 {
    fn min_index_usize() -> usize {
        0
    }
    fn min_index() -> Self {
        0
    }

    fn max_index_usize() -> usize {
        U8_MAX_INDEX_USIZE
    }
    fn max_index() -> Self {
        U8_MAX_INDEX
    }

    fn max_indexable_len() -> usize {
        U8_MAX_INDEXABLE_LEN
    }
    fn from_usize(index: usize) -> Self {
        assert!(index <= Self::max_index_usize());
        index as u8
    }
    fn to_usize(&self) -> usize {
        *self as usize
    }
}

impl Index for NonZeroU8 {
    fn min_index() -> Self {
        NonZeroU8::MIN
    }

    fn max_index_usize() -> usize {
        NON_ZERO_U8_MAX_INDEX_USIZE
    }
    fn max_index() -> Self {
        NON_ZERO_U8_MAX_INDEX
    }

    fn max_indexable_len() -> usize {
        NON_ZERO_USIZE_MAX_INDEXABLE_LEN
    }
    fn from_usize(index: usize) -> Self {
        NonZeroU8::try_from(NonZeroUsize::new(index).unwrap()).unwrap()
    }
    fn to_usize(&self) -> usize {
        self.get() as usize
    }
}

// TODO u16: different on 16 bit and 32+bit
//
// TODO u32: different on 32 bit and 64bit
//
// TODO u64: alias to usize

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
                // The following failed to compile with our crate's feature
                // `_internal_use_allocator_api` (on `nightly`)
                //let vec_deque = ptr::read(&self.vec_deque as *const VecDeque<T, A>);
                //let mut vec_deque =
                //    mem::transmute::<VecDeque<T, A>, VecDeque<MaybeUninit<T>, A>>(vec_deque);

                // TODO is this sound?
                let mut vec_deque =
                    ptr::read(&self.vec_deque as *const _ as *const VecDeque<MaybeUninit<T>, A>);

                vec_deque.push_front(MaybeUninit::uninit());
                vec_deque.push_back(MaybeUninit::new(value));
                let popped = vec_deque.pop_front();
                debug_assert!(popped.is_some());

                // The following caused an error again:
                // let vec_deque = mem::transmute::<_, VecDeque<T, A>>(vec_deque);
                // ptr::write(&mut self.vec_deque as *mut VecDeque<T, A>, vec_deque);

                // TODO is this sound?
                ptr::write(
                    &mut self.vec_deque as *mut _ as *mut VecDeque<MaybeUninit<T>, A>,
                    vec_deque,
                );
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
