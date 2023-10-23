//! Wrappers around [`alloc::alloc::Allocator`] and [`alloc::alloc::Global`], so that we write
//! the same code whether this crate is used with custom allocators (`nightly` as of 2023), or with
//! standard allocator.
//!
//! Restricted functionality, crossing data access with other structures in an `unsafe` way.
//!
//! As of starting this (October 2023), there has been no `calloc` crate on crates.io. If there is
//! one in the future, and if it were used together with this, you can alias.

use alloc::collections::VecDeque as StdVecDeque;
use alloc::vec::Vec as StdVec;
//#[cfg(feature = "use_allocator_api")]
//use alloc::alloc::{Allocator as StdAllocator, Global as StdGlobal};

#[cfg(feature = "use_allocator_api")]
pub use alloc::alloc::{Allocator, Global};

// TODO Consider having a separate module file for non-nightly, and then apply `#[cfg(...)]` above
// the `mod` keyword only.
#[cfg(not(feature = "use_allocator_api"))]
pub trait Allocator {}

#[cfg(not(feature = "use_allocator_api"))]
#[derive(Clone, Copy)]
pub struct Global {}

#[cfg(not(feature = "use_allocator_api"))]
impl Allocator for Global {}

// `A: Allocator` is possible (and required) here with #![feature(lazy_type_alias)] ONLY:
#[cfg(feature = "use_allocator_api")]
pub type Vec<T, A: Allocator = Global> = StdVec<T, A>;

// TODO
//pub type Vec<T, #[cfg(feature = "use_allocator_api")] A = Global> = StdVec<T>;
//
// pub type Vec<T, #[cfg(feature = "use_allocator_api")] A = Global> = StdVec<T, #[cfg(feature = "use_allocator_api")] A>;

struct S<T, #[cfg(feature = "use_allocator_api")] A = Global> {
    t: T,
    #[cfg(feature = "use_allocator_api")]
    a: A,
}
// We COULD have conditionally compiled code within `impl<...>`:
//
// impl <T, #[cfg(feature = "use_allocator_api")] A = Global> S<T> {/*... */}
//
// (though that would complain once that crate feature is enabled).
//
// BUT, we CANNOT have conditionally compiled code within the target type's signature (the type
// being implemented by this `impl`):
//
// impl <T, #[cfg(feature = "use_allocator_api")] A = Global> S<T, #[cfg(feature =
// "use_allocator_api")] A> {/*... */}

// TODO
//
//pub type VecDeque<T = Global> = ...
