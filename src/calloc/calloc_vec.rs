use alloc::collections::VecDeque as StdVecDeque;
use alloc::vec::Vec as StdVec;

use crate::calloc::{Allocator, Global};
#[cfg(not(feature = "_internal_use_allocator_api"))]
use core::marker::PhantomData;
#[cfg(not(feature = "_internal_use_allocator_api"))]
use core::ops::{Deref, DerefMut};

// Alternatively, we could apply
// https://github.com/rust-lang/rfcs/blob/master/text/0213-defaulted-type-params.md BUT it's
// unstable (behind `#![feature(default_type_parameter_fallback)]`). See
// https://github.com/rust-lang/rust/issues/27336 - it has been unstable for 8+ years! See also
// https://faultlore.com/blah/defaults-affect-inference/#solving-hashers-and-allocators-reality-version.
//
// But, this would also need a language feature "ignore/allow unused type alias parameter".

//-------- Vec
/* The following is an alternative to the alias `pub type Vec<T, A: Allocator = Global> = StdVec<T, A>;` (and for the similar alias for VecDeque).
However, the following still caused the same error.

#[cfg(feature = "_internal_use_allocator_api")]
/// For re-export
mod from_std {
    pub use alloc::vec::Vec;
    pub use alloc::collections::VecDeque;
}
#[cfg(feature = "_internal_use_allocator_api")]
pub use from_std::{Vec, VecDeque};
*/
#[cfg(feature = "_internal_use_allocator_api")]
pub type Vec<T, A: Allocator = Global> = StdVec<T, A>;
// --

#[cfg(not(feature = "_internal_use_allocator_api"))]
#[repr(transparent)]
pub struct Vec<T, A: Allocator = Global>(pub StdVec<T>, PhantomData<A>);

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> Deref for Vec<T, A> {
    type Target = StdVec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> DerefMut for Vec<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> From<VecDeque<T, A>> for Vec<T, A> {
    fn from(vec_deque: VecDeque<T, A>) -> Self {
        Self(vec_deque.0.into(), PhantomData)
    }
}
//-------- end of: Vec

//-------- VecDeque
//
// `A: Allocator` is required here with #![feature(lazy_type_alias)] (our feature nightly_lazy_type_alias) ONLY:
#[cfg(feature = "_internal_use_allocator_api")]
pub type VecDeque<T, A: Allocator = Global> = StdVecDeque<T, A>;
// --

#[cfg(not(feature = "_internal_use_allocator_api"))]
#[derive(Debug)]
#[repr(transparent)]
pub struct VecDeque<T, A: Allocator = Global>(StdVecDeque<T>, PhantomData<A>);

// TODO if never used in release, then enable it for debug? Or remove!
//
// #[cfg(all(not(feature = "_internal_use_allocator_api"), test))]
//
// and have with_capacity(...) only.
#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> VecDeque<T, A> {
    pub fn new_in(_alloc: A) -> Self {
        Self(StdVecDeque::new(), PhantomData)
    }
    pub fn new() -> Self {
        Self(StdVecDeque::new(), PhantomData)
    }

    pub fn with_capacity_in(capacity: usize, _alloc: A) -> Self {
        Self(StdVecDeque::with_capacity(capacity), PhantomData)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self(StdVecDeque::with_capacity(capacity), PhantomData)
    }
}

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> Deref for VecDeque<T, A> {
    type Target = StdVecDeque<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> DerefMut for VecDeque<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl<T, A: Allocator> From<Vec<T, A>> for VecDeque<T, A> {
    fn from(v: Vec<T, A>) -> Self {
        Self(v.0.into(), PhantomData)
    }
}
//-------- end of: VecDeque

// TODO REPORT
//
// pub type Vec<T, #[cfg(feature = "_internal_use_allocator_api")] A = Global> =
//StdVec<T>;
//
// pub type Vec<T, #[cfg(feature = "_internal_use_allocator_api")] A = Global> = StdVec<T,
// #[cfg(feature = "_internal_use_allocator_api")] A>;

struct S<T, #[cfg(feature = "_internal_use_allocator_api")] A = Global> {
    t: T,
    #[cfg(feature = "_internal_use_allocator_api")]
    a: A,
}
// We COULD have conditionally compiled code within `impl<...>`:
//
// impl <T, #[cfg(feature = "_internal_use_allocator_api")] A = Global> S<T> {/*... */}
//
// (though that would complain once that crate feature is enabled).
//
// BUT, we CANNOT have conditionally compiled code within the target type's signature (the type
// being implemented by this `impl`):
//
// impl <T, #[cfg(feature = "_internal_use_allocator_api")] A = Global> S<T, #[cfg(feature =
// "_internal_use_allocator_api")] A> {/*... */}

// TODO
//
//pub type VecDeque<T = Global> = ...
