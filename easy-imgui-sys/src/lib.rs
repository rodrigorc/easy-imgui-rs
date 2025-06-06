#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unnecessary_transmutes)]
#![allow(clippy::all)]

use std::ops::{Deref, DerefMut, Index};
use std::slice::SliceIndex;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl<T, I: SliceIndex<[T]>> Index<I> for ImVector<T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        let slice: &[T] = *&self;
        &slice[index]
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

impl<T> DerefMut for ImVector<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            if self.Size == 0 {
                // self.Data may be null, and that will not do for `from_raw_parts`
                &mut []
            } else {
                std::slice::from_raw_parts_mut(self.Data, self.Size as usize)
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

impl<'a, T> IntoIterator for &'a mut ImVector<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.deref_mut().into_iter()
    }
}

#[cfg(target_env = "msvc")]
impl From<ImVec2_rr> for ImVec2 {
    fn from(rr: ImVec2_rr) -> ImVec2 {
        ImVec2 { x: rr.x, y: rr.y }
    }
}
