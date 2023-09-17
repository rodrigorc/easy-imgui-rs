use super::*;

impl Context {
    pub fn style<'a>(&'a mut self) -> StyleMut<'a> {
        let ptr = unsafe {
            &mut *ImGui_GetStyle()
        };
        StyleMut(StylePtr {
            ptr,
        })
    }
}

impl<'ctx, D: 'ctx> Ui<'ctx, D> {
    pub fn style<'a>(&'a mut self) -> Style<'a> {
        let ptr = unsafe {
            &mut *ImGui_GetStyle()
        };
        Style(StylePtr {
            ptr,
        })
    }
}

#[derive(Debug)]
pub struct Style<'a>(StylePtr<'a>);
#[derive(Debug)]
pub struct StyleMut<'a>(StylePtr<'a>);

impl<'a> Deref for Style<'a> {
    type Target = StylePtr<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Deref for StyleMut<'a> {
    type Target = StylePtr<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a> std::ops::DerefMut for StyleMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct StylePtr<'a> {
    ptr: &'a mut ImGuiStyle,
}

impl<'a> StylePtr<'a> {
    pub fn colors_light(&mut self) {
        unsafe {
            ImGui_StyleColorsLight(self.ptr);
        }
    }
    pub fn colors_dark(&mut self) {
        unsafe {
            ImGui_StyleColorsDark(self.ptr);
        }
    }
    pub fn colors_classic(&mut self) {
        unsafe {
            ImGui_StyleColorsClassic(self.ptr);
        }
    }
    pub fn color(&self, id: ColorId) -> Color {
        self.ptr.Colors[id.bits() as usize].into()
    }
    pub fn set_color(&mut self, id: ColorId, color: Color) {
        self.ptr.Colors[id.bits() as usize] = color.into();
    }
}
