#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

use std::ops::Index;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl<T: Into<[f32; 2]>> From<T> for ImVec2 {
    fn from(a: T) -> Self {
        let [x, y] = a.into();
        ImVec2 { x, y }
    }
}

impl<T> Index<usize> for ImVector<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.Size as usize {
            panic!("ImVector out of bounds");
        }
        unsafe {
            &*self.Data.add(index)
        }
    }
}