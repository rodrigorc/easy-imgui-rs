#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

use std::ops::{Deref, Index};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl<T> Index<usize> for ImVector<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.Size as usize {
            panic!("ImVector out of bounds");
        }
        unsafe { &*self.Data.add(index) }
    }
}

impl<T> Deref for ImVector<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe {
            if self.Size == 0 {
                // self.Data may be null, and that will not do for `from_raw_parts`
                &[]
            } else {
                std::slice::from_raw_parts(self.Data, self.Size as usize)
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a ImVector<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.deref().into_iter()
    }
}
