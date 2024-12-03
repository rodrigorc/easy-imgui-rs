/*!
* This crate contains a `Renderer` that uses OpenGL, via the [`glow`] crate, to render an
* [`easy-imgui`](https://docs.rs/easy-imgui) user interface.
*/

pub use easy_imgui;
pub use easy_imgui_sys;

// Handy re-exports of core dependencies
pub use easy_opengl::{self, glow};

mod renderer;
pub use renderer::*;
