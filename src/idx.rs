use core::num::{NonZeroU8, NonZeroUsize};
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
