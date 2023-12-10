# easy-imgui-rs
Build full GUI applications with Rust and [Dear ImGui][dearimgui].

There are several crates in this repository:
 * `easy-imgui-sys`: This is the direct binding of the C++ Dear ImGui library.
 * `easy-imgui`: The main binding of Dear ImGui API.
 * `easy-imgui-renderer`: A UI renderer using OpenGL and [`glow`][glow].
 * `easy-imgui-window`: A fully integrated and easy to use GUI framework based on [`winit`][winit].

[dearimgui]: https://github.com/ocornut/imgui
[glow]: https://github.com/grovesNL/glow
[winit]: https://github.com/rust-windowing/winit

See the simplest of examples at the [example][easy-imgui/examples/] directory.
