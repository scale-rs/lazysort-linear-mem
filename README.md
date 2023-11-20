# Lazy (delayed) Quick Sort with all storage pre-allocated

## Objectives

- No extra allocation (except when built with meticulous checks).
- Leveraging standard Rust
  ([alloc::vec::Vec](https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html#method.to_vec),
  [alloc::collections::VecDeque](https://doc.rust-lang.org/nightly/alloc/collections/vec_deque/struct.VecDeque.html)...)
- Minimal & guard-controlled `unsafe` code and
  [core::mem::MaybeUninit](https://doc.rust-lang.org/nightly/core/mem/union.MaybeUninit.html).

## Initial Limitations

Both of the following limitations will be lifted later:

- Stack depth (recursive).
- Needs `alloc`.
