#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

use std::ops::{Index, Deref};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub type Vector2 = mint::Vector2<f32>;
pub type Vector4 = mint::Vector4<f32>;
pub type Color = Vector4;

impl From<ImVec2> for Vector2 {
    #[inline]
    fn from(v: ImVec2) -> Vector2 {
        Vector2 {
            x: v.x,
            y: v.y,
        }
    }
}
impl From<Vector2> for ImVec2 {
    #[inline]
    fn from(v: Vector2) -> ImVec2 {
        ImVec2 {
            x: v.x,
            y: v.y,
        }
    }
}

impl mint::IntoMint for ImVec2 {
    type MintType = Vector2;
}

impl ImVec2 {
    pub fn zero() -> ImVec2 {
        ImVec2 {
            x: 0.0,
            y: 0.0,
        }
    }
    pub fn new(x: f32, y: f32) -> ImVec2 {
        ImVec2 { x, y }
    }
    pub fn as_vector2(self) -> Vector2 {
        self.into()
    }
}

impl From<ImVec4> for Vector4 {
    #[inline]
    fn from(v: ImVec4) -> Vector4 {
        Vector4 {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }
}
impl From<Vector4> for ImVec4 {
    #[inline]
    fn from(v: Vector4) -> ImVec4 {
        ImVec4 {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }
}

impl mint::IntoMint for ImVec4 {
    type MintType = Vector4;
}

impl ImVec4 {
    pub fn zero() -> ImVec4 {
        ImVec4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        }
    }
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> ImVec4 {
        ImVec4 { x, y, z, w }
    }
    pub fn as_vector4(self) -> Vector4 {
        self.into()
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
