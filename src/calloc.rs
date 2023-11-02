//! Re-exports/substitutes for [`alloc::alloc::Allocator`] &[`alloc::alloc::Global`], and
//! allocation-enabled [`alloc::vec::Vec`] & [`alloc::collections::VecDeque`], so that we write the
//! same code whether this crate is used with custom allocators (`nightly`-only as of 2023), or with
//! standard allocator (`stable`/`beta` as of 2023).
//!
//! Restricted functionality, crossing data access with other structures in an `unsafe` way.
//!
//! As of starting this (October 2023), there has been no `calloc` crate on crates.io. If there is
//! one in the future, and if it were used together with this, you can alias.

#[cfg(not(feature = "_internal_use_allocator_api"))]
use core::marker::PhantomData;

#[cfg(not(feature = "_internal_use_allocator_api"))]
use core::ops::{Deref, DerefMut};

//#[cfg(feature = "_internal_use_allocator_api")]
//use alloc::alloc::{Allocator as StdAllocator, Global as StdGlobal};

//-------- Allocator, Global
#[cfg(feature = "_internal_use_allocator_api")]
pub use alloc::alloc::{Allocator, Global};
// --

// TODO Consider having a separate module file for non-nightly, and then apply `#[cfg(...)]` above
// the `mod` keyword only.
#[cfg(not(feature = "_internal_use_allocator_api"))]
pub trait Allocator {}

#[cfg(not(feature = "_internal_use_allocator_api"))]
#[derive(Clone, Copy, Debug)]
pub struct Global {}

#[cfg(not(feature = "_internal_use_allocator_api"))]
impl Allocator for Global {}
// TODO Drop - here or elsewhere?
//-------- end of: Allocator, Global

#[cfg(feature = "alloc")]
pub mod calloc_vec;
