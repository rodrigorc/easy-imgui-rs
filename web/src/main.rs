use easy_imgui as imgui;
use easy_imgui_sys::*;

fn main() {
    println!("Hello, world!");
    let mut imgui = unsafe { imgui::Context::new() };
    println!("A");
    unsafe {
        imgui.set_current();
    println!("B");
        imgui.set_size([400.0, 400.0], 1.0);
    println!("C");

        ImGui_NewFrame();
    println!("D");
        ImGui_ShowDemoWindow(std::ptr::null_mut());
    println!("E");
        ImGui_Render();
    println!("F");
    }
    println!("Bye!");
}
