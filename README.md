# easy-imgui-rs

[![build](https://github.com/rodrigorc/easy-imgui-rs/actions/workflows/build.yaml/badge.svg?branch=main)](https://github.com/rodrigorc/easy-imgui-rs/actions/workflows/build.yaml)

Build full GUI applications with Rust and [Dear ImGui][dearimgui]. It currently uses version v1.92.4.

There are several crates in this repository:
 * [`easy-imgui-sys`](https://crates.io/crates/easy-imgui-sys): This is the direct binding of the C++ Dear ImGui library.
 * [`easy-imgui`](https://crates.io/crates/easy-imgui): The main binding of Dear ImGui API.
 * [`easy-imgui-renderer`](https://crates.io/crates/easy-imgui-renderer): A UI renderer using OpenGL and [`glow`][glow].
 * [`easy-imgui-window`](https://crates.io/crates/easy-imgui-window): A fully integrated and easy to use GUI framework based on [`winit`][winit].

See some examples at the [examples](https://github.com/rodrigorc/easy-imgui-rs/tree/main/easy-imgui-window/examples) directory. The simplest one is just a few lines of code:
```rust
use easy_imgui_window::{MainWindow, MainWindowWithRenderer,
    winit::event_loop::EventLoopBuilder,
    easy_imgui as imgui,
};

fn main() {
    let event_loop = EventLoopBuilder::new().build().unwrap();
    let main_window = MainWindow::new(&event_loop, "Example").unwrap();
    let mut window = MainWindowWithRenderer::new(main_window);

    let mut app = App;

    event_loop.run(move |event, w| {
        let res = window.do_event(&mut app, &event, w);
        if res.is_break() {
            w.exit();
        }
    }).unwrap();
}

struct App;

impl imgui::UiBuilder for App {
    fn do_ui(&mut self, ui: &imgui::Ui<Self>) {
        ui.show_demo_window(None);
    }
}
```

[dearimgui]: https://github.com/ocornut/imgui
[glow]: https://github.com/grovesNL/glow
[winit]: https://github.com/rust-windowing/winit
