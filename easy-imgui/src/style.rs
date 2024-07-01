use super::*;

impl CurrentContext<'_> {
    pub fn style<'s>(&'s mut self) -> &'s mut Style {
        unsafe {
            let ptr = ImGui_GetStyle();
            &mut *(ptr as *mut Style)
        }
    }
}

impl<A> Ui<A> {
    pub fn style<'s>(&'s self) -> &'s Style {
        unsafe {
            let ptr = ImGui_GetStyle();
            &*(ptr as *const Style)
        }
    }
}

/// A wrapper for the `ImGuiStyle` type.
///
/// It can be deref-ed directly into a `ImGuiStyle` reference.
#[derive(Debug)]
#[repr(transparent)]
pub struct Style(ImGuiStyle);

impl<'a> std::ops::Deref for Style {
    type Target = ImGuiStyle;
    fn deref(&self) -> &ImGuiStyle {
        self.get()
    }
}

impl<'a> std::ops::DerefMut for Style {
    fn deref_mut(&mut self) -> &mut ImGuiStyle {
        self.get_mut()
    }
}

impl Style {
    pub fn get(&self) -> &ImGuiStyle {
        &self.0
    }
    pub fn get_mut(&mut self) -> &mut ImGuiStyle {
        &mut self.0
    }
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
        self.0.Colors[id.bits() as usize].into()
    }
    pub fn set_color(&mut self, id: ColorId, color: Color) {
        self.0.Colors[id.bits() as usize] = color.into();
    }
    pub fn color_alpha(&self, id: ColorId, alpha_mul: f32) -> Color {
        let mut c = self.color(id);
        let a = self.Alpha;
        c.a *= a * alpha_mul;
        c
    }
}
