#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

use std::ops::{Index, Deref};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl<T: Into<[f32; 2]>> From<T> for ImVec2 {
    fn from(a: T) -> Self {
        let [x, y] = a.into();
        ImVec2 { x, y }
    }
}

impl ImVec2 {
    pub fn as_array(&self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl<T: Into<[f32; 4]>> From<T> for ImVec4 {
    fn from(a: T) -> Self {
        let [x, y, z, w] = a.into();
        ImVec4 { x, y, z, w }
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


impl<T> Deref for ImVector<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.Data, self.Size as usize)
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

 /// Color is stored as [r, g, b, a]
pub type Color = [u8; 4];

pub trait IntoColor: Sized {
    fn color(self) -> u32;
    fn color_vec(self) -> ImVec4 {
        let c = self.color();
        unsafe {
            ImGui_ColorConvertU32ToFloat4(c)
        }
    }
}

impl<T: Into<Color>> IntoColor for T {
    fn color(self) -> u32 {
        u32::from_ne_bytes(self.into())
    }
}
