use super::{FontBaked, PixelImage, SubPixelImage, Vector2, vec2};
use bitflags::bitflags;
use easy_imgui_sys::*;
use image::GenericImage;

/// Trait that implements a custom font loader.
pub trait GlyphLoader {
    /// Return true if this font loader contains this character.
    fn contains_glyph(&mut self, codepoint: char) -> bool;
    /// Try to load a glyph.
    ///
    /// Refer to [`GlyphLoaderArg`] for details.
    fn load_glyph(&mut self, arg: GlyphLoaderArg<'_>);
    /// Initialize the parameters of the corresponding `FontBaked`.
    ///
    /// Not needed if this custom loader is part of a font collection and is not the first one.
    fn font_baked_init(&mut self, _baked: &mut FontBaked) {}
    /// The font baked is about to be destroyed.
    ///
    /// Rarely useful, maybe if you had some raw resource stored in the `baked`.
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
    unsafe {
        let ptr = (*src).FontLoaderData;
        if !ptr.is_null() {
            return false;
        }
        (*src).FontLoaderData = std::mem::replace(&mut (*src).FontData, std::ptr::null_mut());
        true
    }
}

unsafe extern "C" fn font_src_destroy(_atlas: *mut ImFontAtlas, src: *mut ImFontConfig) {
    unsafe {
        let ptr = (*src).FontLoaderData;
        if ptr.is_null() {
            return;
        }
        let ptr = ptr as *mut BoxGlyphLoader;
        drop(Box::from_raw(ptr));
        (*src).FontLoaderData = std::ptr::null_mut();
    }
}

unsafe extern "C" fn font_src_contains_glyph(
    _atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    codepoint: ImWchar,
) -> bool {
    unsafe {
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
}

unsafe extern "C" fn font_baked_load_glyph(
    atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    baked: *mut ImFontBaked,
    _loader_data_for_baked_src: *mut ::std::os::raw::c_void,
    codepoint: ImWchar,
    out_glyph: *mut ImFontGlyph,
    out_advance_x: *mut f32,
) -> bool {
    unsafe {
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
        let mut oversample_h = 0;
        let mut oversample_v = 0;
        ImFontAtlasBuildGetOversampleFactors(src, baked, &mut oversample_h, &mut oversample_v);
        let rasterizer_density = src.RasterizerDensity * baked.RasterizerDensity;
        let mut result = false;

        let output = if out_advance_x.is_null() {
            GlyphLoaderResult::Glyph(&mut *out_glyph)
        } else {
            GlyphLoaderResult::AdvanceX(&mut *out_advance_x)
        };
        let arg = GlyphLoaderArg {
            codepoint,
            oversample: vec2(oversample_h as f32, oversample_v as f32),
            rasterizer_density,
            result: &mut result,
            atlas,
            src,
            baked,
            output,
        };
        ldr.load_glyph(arg);
        result
    }
}

unsafe extern "C" fn font_baked_init(
    _atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    baked: *mut ImFontBaked,
    _loader_data_for_baked_src: *mut ::std::os::raw::c_void,
) -> bool {
    unsafe {
        let ptr = (*src).FontLoaderData;
        if ptr.is_null() {
            return false;
        }
        let ptr = ptr as *mut BoxGlyphLoader;
        let ldr = &mut *ptr;

        ldr.font_baked_init(FontBaked::cast_mut(&mut *baked));
        true
    }
}

unsafe extern "C" fn font_baked_destroy(
    _atlas: *mut ImFontAtlas,
    src: *mut ImFontConfig,
    baked: *mut ImFontBaked,
    _loader_data_for_baked_src: *mut ::std::os::raw::c_void,
) {
    unsafe {
        let ptr = (*src).FontLoaderData;
        if ptr.is_null() {
            return;
        }
        let ptr = ptr as *mut BoxGlyphLoader;
        let ldr = &mut *ptr;

        ldr.font_baked_destroy(FontBaked::cast_mut(&mut *baked));
    }
}

enum GlyphLoaderResult<'a> {
    Glyph(&'a mut ImFontGlyph),
    AdvanceX(&'a mut f32),
}

/// Arguments for [`GlyphLoader::load_glyph`].
///
/// Having a struct instead of a collection of parameters makes it easier to write and use.
pub struct GlyphLoaderArg<'a> {
    codepoint: char,
    // This are integer in DearImGui, but why not fractions?
    oversample: Vector2,
    rasterizer_density: f32,
    result: &'a mut bool,

    atlas: &'a mut ImFontAtlas,
    src: &'a mut ImFontConfig,
    baked: &'a mut ImFontBaked,
    output: GlyphLoaderResult<'a>,
}

bitflags! {
    /// Flags to modify the behavior of `GlyphLoaderArg::build()`.
    #[derive(Copy, Clone, Debug)]
    pub struct GlyphBuildFlags: u32 {
        /// Sets the oversample to (1, 1) before building the image.
        const IGNORE_OVERSAMPLE = 1;
        /// Sets dpi_density to 1.0 before building the image.
        const IGNORE_DPI = 2;
        /// The size passed in already scaled to the final size.
        const PRESCALED_SIZE = 4;
        /// Sets all the scaling factors to 1. This will make the final image to the size passed to `build()`.
        ///
        /// Note that setting `PRESCALED_SIZE` and `IGNORE_SCALE` does nothing.
        const IGNORE_SCALE = Self::IGNORE_OVERSAMPLE.bits() | Self::IGNORE_DPI.bits();
    }
}

impl GlyphLoaderArg<'_> {
    /// The character to be loaded.
    pub fn codepoint(&self) -> char {
        self.codepoint
    }
    /// The current size of the font requested.
    pub fn font_size(&self) -> f32 {
        self.baked.Size
    }
    /// Gets the rasterizer density (DPI) of the renderer that requests this glyph.
    ///
    /// It is usually 1.0, but in hiDPI settings it may be 2.0 (or any other value, actually).
    pub fn dpi_density(&self) -> f32 {
        self.rasterizer_density
    }
    /// Sets the rasterizer density.
    ///
    /// If you can't (or don't want to) support hiDPI environments you can disable it by either
    /// setting the `GlyphBuildFlags::IGNORE_DPI` or by calling this function.
    /// You can set it to values other than 1.0, but the usefulness is limited.
    pub fn set_dpi_density(&mut self, scale: f32) {
        self.rasterizer_density = scale;
    }
    /// Gets the X/Y oversample factor.
    ///
    /// This is usually (1.0, 1.0), but for small fonts, it may be (2.0, 1.0).
    /// If so, you should render your glyph scaled by these factors in X and Y.
    pub fn oversample(&self) -> Vector2 {
        self.oversample
    }
    /// Sets the X/Y oversample factor.
    ///
    /// If you can't (or don't want to) support oversampling you can disable it by either
    /// setting the `GlyphBuildFlags::IGNORE_OVERSAMPLE` or by calling this function.
    pub fn set_oversample(&mut self, oversample: Vector2) {
        self.oversample = oversample;
    }
    /// Returns the "only advance X" flag.
    ///
    /// If this is true, when you call `build`, the `draw` callback will not actually be called:
    /// only the `advance_x` parameter will be used.
    ///
    /// This is used by Dear ImGui when using very big fonts to compute the position of the glyphs
    /// before rendering them, to save space in the texture atlas.
    ///
    /// You can safely ignore this, if you don't need to micro-optimize the loading of very big
    /// custom fonts.
    pub fn only_advance_x(&self) -> bool {
        matches!(self.output, GlyphLoaderResult::AdvanceX(_))
    }
    /// Builds the requested glyph.
    ///
    /// `origin` is the offset of the origin of the glyph, by default it will be just over the
    /// baseline.
    /// `size` is the size of the glyph, when drawn to the screen.
    /// `advance_x` is how many pixels this glyph occupies when part of a string.
    /// `flags`: how to interpret the size and scale the bitmap.
    /// `draw`: callback that actually draws the glyph.
    ///
    /// Note that `draw()` may not be actually called every time. You can use `only_advance_x()` to
    /// detect this case, if you need it.
    pub fn build(
        mut self,
        origin: Vector2,
        mut size: Vector2,
        advance_x: f32,
        flags: GlyphBuildFlags,
        draw: impl FnOnce(&mut SubPixelImage<'_, '_>),
    ) {
        match self.output {
            GlyphLoaderResult::AdvanceX(out_advance_x) => {
                *out_advance_x = advance_x;
            }
            GlyphLoaderResult::Glyph(out_glyph) => {
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

                let ref_size = unsafe { (*(&(*self.baked.OwnerFont).Sources)[0]).SizePixels };
                let offsets_scale = if ref_size != 0.0 {
                    self.baked.Size / ref_size
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

                out_glyph.set_Codepoint(self.codepoint as u32);
                out_glyph.AdvanceX = advance_x;
                out_glyph.X0 = font_off_x + origin.x;
                out_glyph.Y0 = font_off_y + origin.y;
                out_glyph.X1 = out_glyph.X0 + size.x;
                out_glyph.Y1 = out_glyph.Y0 + size.y;
                out_glyph.set_Visible(1);
                out_glyph.PackId = pack_id;

                let mut pixels =
                    image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::new(bmp_size_x, bmp_size_y)
                        .into_raw();
                let mut image = PixelImage::from_raw(bmp_size_x, bmp_size_y, &mut pixels).unwrap();
                draw(&mut image.sub_image(0, 0, bmp_size_x, bmp_size_y));
                unsafe {
                    ImFontAtlasBakedSetFontGlyphBitmap(
                        self.atlas,
                        self.baked,
                        self.src,
                        out_glyph,
                        r,
                        pixels.as_ptr(),
                        ImTextureFormat::ImTextureFormat_RGBA32,
                        4 * bmp_size_x as i32,
                    );
                }
            }
        }
        *self.result = true;
    }
}
