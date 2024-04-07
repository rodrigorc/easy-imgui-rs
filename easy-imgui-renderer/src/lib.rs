/*!
* This crate contains a `Renderer` that uses OpenGL, via the [`glow`] crate, to render an
* [`easy-imgui`](https://docs.rs/easy-imgui) user interface.
*/

pub mod glr;
mod renderer;
pub use easy_imgui;
pub use easy_imgui_sys;
pub use glow;

pub use renderer::*;

// These macros should be in glr, but hey...

#[macro_export]
macro_rules! uniform {
    (
        $(
            $(#[$a:meta])* $v:vis struct $name:ident {
                $(
                    $fv:vis $f:ident : $ft:tt
                ),*
                $(,)?
            }
        )*
    ) => {
        $(
            $(#[$a])* $v struct $name {
                $(
                    $fv $f: $ft ,
                )*
            }
            impl $crate::glr::UniformProvider for $name {
                fn apply(&self, gl: &$crate::glr::GlContext, u: &$crate::glr::Uniform) {
                    let name = u.name();
                    $(
                        if name == $crate::uniform!{ @NAME $f: $ft }  {
                            self.$f.apply(gl, u.location());
                            return;
                        }
                    )*
                }
            }
        )*
    };
    (@NAME $f:ident : [ $ft:ty; $n:literal ]) => { concat!(stringify!($f), "[0]") };
    (@NAME $f:ident : $ft:ty) => { stringify!($f) };
}

#[macro_export]
macro_rules! attrib {
    (
        $(
            $(#[$a:meta])* $v:vis struct $name:ident {
                $(
                    $fv:vis $f:ident : $ft:ty
                ),*
                $(,)?
            }
        )*
    ) => {
        $(
            $(#[$a])* $v struct $name {
                $(
                    $fv $f: $ft ,
                )*
            }
            unsafe impl $crate::glr::AttribProvider for $name {
                fn apply(gl: &$crate::glr::GlContext, a: &$crate::glr::Attribute) -> Option<(usize, u32, usize)> {
                    let name = a.name();
                    $(
                        if name == stringify!($f) {
                            let (n, t) = <$ft as $crate::glr::AttribField>::detail();
                            return Some((n, t, memoffset::offset_of!($name, $f)));
                        }
                    )*
                    None
                }
            }
        )*
    }
}
