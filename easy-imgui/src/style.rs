use super::*;

transparent_mut! {
    /// A wrapper for the `ImGuiStyle` type.
    ///
    /// It can be deref-ed directly into a `ImGuiStyle` reference.
    #[derive(Debug)]
    pub struct Style(ImGuiStyle);
}

impl Style {
    pub fn set_colors_light(&mut self) {
        unsafe {
            ImGui_StyleColorsLight(&mut self.0);
        }
    }
    pub fn set_colors_dark(&mut self) {
        unsafe {
            ImGui_StyleColorsDark(&mut self.0);
        }
    }
    pub fn set_colors_classic(&mut self) {
        unsafe {
            ImGui_StyleColorsClassic(&mut self.0);
        }
    }
    pub fn color(&self, id: ColorId) -> Color {
        self.Colors[id.bits() as usize].into()
    }
    pub fn set_color(&mut self, id: ColorId, color: Color) {
        self.Colors[id.bits() as usize] = color.into();
    }
    pub fn color_alpha(&self, id: ColorId, alpha_mul: f32) -> Color {
        let mut c = self.color(id);
        let a = self.Alpha;
        c.a *= a * alpha_mul;
        c
    }
}
