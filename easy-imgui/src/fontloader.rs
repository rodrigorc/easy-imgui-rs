use super::{vec2, FontBaked, PixelImage, SubPixelImage, Vector2};
use bitflags::bitflags;
use easy_imgui_sys::*;
use image::GenericImage;

pub trait GlyphLoader {
    fn contains_glyph(&mut self, codepoint: char) -> bool;
    fn load_glyph(&mut self, arg: GlyphLoaderArg<'_>);
    fn font_baked_init(&mut self, _baked: &mut FontBaked) {}
    fn font_baked_destroy(&mut self, _baked: &mut FontBaked) {}
}

pub type BoxGlyphLoader = Box<dyn GlyphLoader + 'static>;

/// Newtype for the one and only FONT_LOADER.
pub struct FontLoader(pub ImFontLoader);
unsafe impl Sync for FontLoader {}

pub static FONT_LOADER: FontLoader = FontLoader::new();

impl FontLoader {
    const fn new() -> Self {
        let inner = ImFontLoader {
            Name: c"easy_imgui::FontLoader".as_ptr(),
            LoaderInit: None,
            LoaderShutdown: None,
            FontSrcInit: Some(font_src_init),
            FontSrcDestroy: Some(font_src_destroy),
            FontSrcContainsGlyph: Some(font_src_contains_glyph),
            FontBakedInit: Some(font_baked_init),
            FontBakedDestroy: Some(font_baked_destroy),
            FontBakedLoadGlyph: Some(font_baked_load_glyph),
            FontBakedSrcLoaderDataSize: 0,
        };
        FontLoader(inner)
    }
}

unsafe extern "C" fn font_src_init(_atlas: *mut ImFontAtlas, src: *mut ImFontConfig) -> bool {
    let ptr = (*src).FontLoaderData;
    if !ptr.is_null() {
        return false;
    }
    (*src).FontLoaderData = std::mem::replace(&mut (*src).FontData, std::ptr::null_mut());
    println!("font_src_init {:?}", (*src).FontLoaderData);
    true
}

unsafe extern "C" fn font_src_destroy(_atlas: *mut ImFontAtlas, src: *mut ImFontConfig) {
    let ptr = (*src).FontLoaderData;
    if ptr.is_null() {
        return;
    }
    let ptr = ptr as *mut BoxGlyphLoader;
    drop(Box::from_raw(ptr));
    (*src).FontLoaderData = std::ptr::null_mut();
}

unsafe extern "C" fn font_src_contains_glyph(
    _atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    codepoint: ImWchar,
) -> bool {
    let ptr = (*src).FontLoaderData;
    if ptr.is_null() {
        return false;
    }
    let Some(c) = char::from_u32(codepoint) else {
        return false;
    };
    let ptr = ptr as *mut BoxGlyphLoader;
    let ldr = &mut *ptr;
    ldr.contains_glyph(c)
}

unsafe extern "C" fn font_baked_load_glyph(
    atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    baked: *mut ImFontBaked,
    _loader_data_for_baked_src: *mut ::std::os::raw::c_void,
    codepoint: ImWchar,
    out_glyph: *mut ImFontGlyph,
) -> bool {
    let ptr = (*src).FontLoaderData;
    if ptr.is_null() {
        return false;
    }
    let Some(codepoint) = char::from_u32(codepoint) else {
        return false;
    };
    let ptr = ptr as *mut BoxGlyphLoader;
    let ldr = &mut *ptr;

    let atlas = &mut *atlas;
    let baked = &mut *baked;
    let src = &mut *src;
    let out_glyph = &mut *out_glyph;

    let mut oversample_h = 0;
    let mut oversample_v = 0;
    ImFontAtlasBuildGetOversampleFactors(src, baked, &mut oversample_h, &mut oversample_v);
    let rasterizer_density = src.RasterizerDensity * baked.RasterizerDensity;

    let mut result = false;
    let arg = GlyphLoaderArg {
        codepoint,
        oversample: vec2(oversample_h as f32, oversample_v as f32),
        rasterizer_density,
        result: &mut result,
        atlas,
        src,
        baked,
        out_glyph,
    };
    ldr.load_glyph(arg);
    result
}

unsafe extern "C" fn font_baked_init(
    _atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    baked: *mut ImFontBaked,
    _loader_data_for_baked_src: *mut ::std::os::raw::c_void,
) -> bool {
    let ptr = (*src).FontLoaderData;
    if ptr.is_null() {
        return false;
    }
    let ptr = ptr as *mut BoxGlyphLoader;
    let ldr = &mut *ptr;

    ldr.font_baked_init(FontBaked::cast_mut(&mut *baked));
    true
}

unsafe extern "C" fn font_baked_destroy(
    _atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    baked: *mut ImFontBaked,
    _loader_data_for_baked_src: *mut ::std::os::raw::c_void,
) {
    let ptr = (*src).FontLoaderData;
    if ptr.is_null() {
        return;
    }
    let ptr = ptr as *mut BoxGlyphLoader;
    let ldr = &mut *ptr;

    ldr.font_baked_destroy(FontBaked::cast_mut(&mut *baked));
}

pub struct GlyphLoaderArg<'a> {
    codepoint: char,
    // This are integer in DearImGui, but why not fractions?
    oversample: Vector2,
    rasterizer_density: f32,
    result: &'a mut bool,

    atlas: &'a mut ImFontAtlas,
    src: &'a mut ImFontConfig,
    baked: &'a mut ImFontBaked,
    out_glyph: &'a mut ImFontGlyph,
}

bitflags! {
    /// Flags to modify the behavior of `GlyphLoaderArg::build()`.
    #[derive(Copy, Clone, Debug)]
    pub struct GlyphBuildFlags: u32 {
        // Sets the oversample to (1, 1) before building the image.
        const IGNORE_OVERSAMPLE = 1;
        /// Sets dpi_density to 1.0 before building the image.
        const IGNORE_DPI = 2;
        /// The size passed in already scaled to the final size.
        const PRESCALED_SIZE = 4;
        /// Sets all the scaling factors to 1. This will make the final image to the size passed to `build)`.
        ///
        /// Setting `PRESCALED_SIZE` and `IGNORE_SCALE` does nothing.
        const IGNORE_SCALE = Self::IGNORE_OVERSAMPLE.bits() | Self::IGNORE_DPI.bits();
    }
}

impl GlyphLoaderArg<'_> {
    pub fn codepoint(&self) -> char {
        self.codepoint
    }
    pub fn font_size(&self) -> f32 {
        self.baked.Size
    }
    pub fn dpi_density(&self) -> f32 {
        self.rasterizer_density
    }
    pub fn set_dpi_density(&mut self, scale: f32) {
        self.rasterizer_density = scale;
    }
    pub fn oversample(&self) -> Vector2 {
        self.oversample
    }
    pub fn set_oversample(&mut self, oversample: Vector2) {
        self.oversample = oversample;
    }
    pub fn build(
        mut self,
        origin: Vector2,
        mut size: Vector2,
        advance_x: f32,
        flags: GlyphBuildFlags,
        draw: impl FnOnce(&mut SubPixelImage<'_, '_>),
    ) {
        if flags.contains(GlyphBuildFlags::IGNORE_DPI) {
            self.rasterizer_density = 1.0;
        }
        if flags.contains(GlyphBuildFlags::IGNORE_OVERSAMPLE) {
            self.oversample = vec2(1.0, 1.0);
        }

        let scale_for_raster = self.rasterizer_density * self.oversample;

        let (bmp_size_x, bmp_size_y);

        if flags.contains(GlyphBuildFlags::PRESCALED_SIZE) {
            bmp_size_x = size.x.round() as u32;
            bmp_size_y = size.y.round() as u32;
            size.x /= scale_for_raster.x;
            size.y /= scale_for_raster.y;
        } else {
            bmp_size_x = (size.x * scale_for_raster.x).round() as u32;
            bmp_size_y = (size.y * scale_for_raster.y).round() as u32;
        }

        let pack_id = unsafe {
            ImFontAtlasPackAddRect(
                self.atlas,
                bmp_size_x as i32,
                bmp_size_y as i32,
                std::ptr::null_mut(),
            )
        };
        if pack_id == ImFontAtlasRectId_Invalid {
            return;
        }
        let r = unsafe {
            let r = ImFontAtlasPackGetRect(self.atlas, pack_id);
            &mut *r
        };

        let ref_size = unsafe { (*(&(*self.baked.ContainerFont).Sources)[0]).SizePixels };
        let offsets_scale = if ref_size != 0.0 {
            self.font_size() / ref_size
        } else {
            1.0
        };
        let mut font_off_x = self.src.GlyphOffset.x * offsets_scale;
        let mut font_off_y = self.src.GlyphOffset.y * offsets_scale;
        if self.src.PixelSnapH {
            font_off_x = font_off_x.round();
        }
        if self.src.PixelSnapV {
            font_off_y = font_off_y.round();
        }

        self.out_glyph.set_Codepoint(self.codepoint as u32);
        self.out_glyph.AdvanceX = advance_x;
        self.out_glyph.X0 = font_off_x + origin.x;
        self.out_glyph.Y0 = font_off_y + origin.y;
        self.out_glyph.X1 = self.out_glyph.X0 + size.x;
        self.out_glyph.Y1 = self.out_glyph.Y0 + size.y;
        self.out_glyph.set_Visible(1);
        self.out_glyph.PackId = pack_id;

        let mut pixels =
            image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(bmp_size_x, bmp_size_y).into_raw();
        let mut image = PixelImage::from_raw(bmp_size_x, bmp_size_y, &mut pixels).unwrap();
        draw(&mut image.sub_image(0, 0, bmp_size_x, bmp_size_y));
        unsafe {
            ImFontAtlasBakedSetFontGlyphBitmap(
                self.atlas,
                self.baked,
                self.src,
                self.out_glyph,
                r,
                pixels.as_ptr(),
                ImTextureFormat::ImTextureFormat_RGBA32,
                4 * bmp_size_x as i32,
            );
        }
        *self.result = true;
    }
}
