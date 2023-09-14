#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::all)]

use std::ops::{Index, Deref};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub type Vector2 = mint::Vector2<f32>;
pub type Color = mint::Vector4<f32>;

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


/*
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
}*/
