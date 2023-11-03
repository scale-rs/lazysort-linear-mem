#![no_std]
#![cfg_attr(feature = "_internal_use_allocator_api", feature(allocator_api))]
#![allow(incomplete_features)]
#![cfg_attr(not(feature = "nightly_lazy_type_alias"), allow(type_alias_bounds))]
#![cfg_attr(feature = "nightly_lazy_type_alias", feature(lazy_type_alias))]
#![cfg_attr(feature = "nightly_strict_provenance", feature(strict_provenance))]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use calloc::{Allocator, Global};

use core::{mem, ops::Deref};
//use cross;

pub mod calloc;
mod idx;
mod store;

#[cfg(feature = "alloc")]
mod lib_vec;

#[cfg(test)]
mod test {
    #[test]
    fn convert_not_invoking_drop() {}
}

/// For ensuring we use the result returned from closures.
#[must_use]
#[repr(transparent)]
struct MustUse<T>(T);

/// Generate a new closure whose result is `#[must_use]`. Should be zero-cost.
#[inline(always)]
fn make_consume_closure_must_use_result<T, CONSUME>(
    mut consume: CONSUME,
) -> impl FnMut(usize, T) -> MustUse<bool>
where
    CONSUME: FnMut(usize, T) -> bool,
{
    move |idx, value| MustUse(consume(idx, value))
}
