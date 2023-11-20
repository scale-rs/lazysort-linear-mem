#[cfg(feature = "alloc")]
use crate::calloc::calloc_vec::{Vec, VecDeque};
use crate::calloc::{Allocator, Global};

pub trait ReDeque<T> {
    type Veccy: ReVec<T, Deqqy = Self>;

    unsafe fn to_veccies(&mut self) -> (Self::Veccy, Self::Veccy);
}

pub trait ReVec<T> {
    type Deqqy: ReDeque<T, Veccy = Self>;

    unsafe fn to_deqqy(&mut self) -> Self::Deqqy;
}

#[cfg(feature = "alloc")]
impl<T, A: Allocator> ReDeque<T> for VecDeque<T, A> {
    type Veccy = Vec<T, A>;

    unsafe fn to_veccies(&mut self) -> (Self::Veccy, Self::Veccy) {
        loop {}
    }
}

#[cfg(feature = "alloc")]
impl<T, A: Allocator> ReVec<T> for Vec<T, A> {
    type Deqqy = VecDeque<T, A>;

    unsafe fn to_deqqy(&mut self) -> Self::Deqqy {
        loop {}
    }
}
