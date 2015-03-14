//! Module for creating text textures.

use common::color::Color4;
use gl;
use sdl2_sys::pixels::{SDL_Color,SDL_PIXELFORMAT_ARGB8888};
use sdl2_sys::surface::SDL_Surface;
use sdl2_sys::surface;
use std::ffi::CString;
use std::path::Path;
use yaglw::gl_context::GLContext;
use yaglw::texture::Texture2D;

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[allow(missing_docs)]
pub mod ffi {
  extern crate libc;

  use sdl2_sys::pixels::SDL_Color;
  use sdl2_sys::surface::SDL_Surface;

  pub use self::libc::{c_int, c_char, c_void, c_long};

  pub type TTF_Font = c_void;

  pub type TTF_StyleFlag = c_int;
  pub static TTF_STYLE_NORMAL: TTF_StyleFlag = 0x00;
  pub static TTF_STYLE_BOLD: TTF_StyleFlag = 0x01;
  pub static TTF_STYLE_ITALIC: TTF_StyleFlag = 0x02;
  pub static TTF_STYLE_UNDERLINE: c_int = 0x04;
  pub static TTF_STYLE_STRIKETHROUGH: c_int = 0x08;

  pub type TTF_Hinting = c_int;
  pub static TTF_HINTING_NORMAL: TTF_Hinting = 0;
  pub static TTF_HINTING_LIGHT: TTF_Hinting = 1;
  pub static TTF_HINTING_MONO: TTF_Hinting = 2;
  pub static TTF_HINTING_NONE: TTF_Hinting = 3;

  #[link(name="SDL2_ttf")]
  extern "C" {
    pub fn TTF_Init() -> c_int;
    pub fn TTF_WasInit() -> c_int;
    pub fn TTF_Quit();
    pub fn TTF_OpenFont(file: *const c_char, ptsize: c_int) -> *mut TTF_Font;
    pub fn TTF_OpenFontIndex(file: *const c_char, ptsize: c_int, index: c_long) -> *mut TTF_Font;
    pub fn TTF_CloseFont(font: *mut TTF_Font);

    pub fn TTF_GetFontStyle(font: *const TTF_Font) -> TTF_StyleFlag;
    pub fn TTF_SetFontStyle(font: *mut TTF_Font, style: TTF_StyleFlag);
    pub fn TTF_GetFontOutline(font: *const TTF_Font) -> c_int;
    pub fn TTF_SetFontOutline(font: *mut TTF_Font, outline: c_int);
    pub fn TTF_GetFontHinting(font: *const TTF_Font) -> TTF_Hinting;
    pub fn TTF_SetFontHinting(font: *mut TTF_Font, hinting: TTF_Hinting);
    pub fn TTF_GetFontKerning(font: *const TTF_Font) -> c_int;
    pub fn TTF_SetFontKerning(font: *mut TTF_Font, kerning: c_int);
    pub fn TTF_FontHeight(font: *const TTF_Font) -> c_int;
    pub fn TTF_FontAscent(font: *const TTF_Font) -> c_int;
    pub fn TTF_FontDescent(font: *const TTF_Font) -> c_int;
    pub fn TTF_FontLineSkip(font: *const TTF_Font) -> c_int;
    pub fn TTF_FontFaces(font: *const TTF_Font) -> c_long;
    pub fn TTF_FontFaceIsFixedWidth(font: *const TTF_Font) -> c_int;
    pub fn TTF_FontFaceFamilyName(font: *const TTF_Font) -> *const c_char;
    pub fn TTF_GlyphIsProvided(font: *const TTF_Font, glyph: u16) -> c_int;
    pub fn TTF_GlyphMetrics(font: *const TTF_Font, glyph: u16, minx: *mut c_int, maxx: *mut c_int, miny: *mut c_int, maxy: *mut c_int, advance: *mut c_int) -> c_int;
    pub fn TTF_SizeUTF8(font: *mut TTF_Font, text: *const c_char, w: *mut c_int, h: *mut c_int) -> c_int;

    pub fn TTF_RenderUTF8_Solid(font: *const TTF_Font, text: *const c_char, fg: SDL_Color) -> *mut SDL_Surface;
    pub fn TTF_RenderUTF8_Shaded(font:  *const TTF_Font, text: *const c_char, fg: SDL_Color, bg: SDL_Color) -> *mut SDL_Surface;
    pub fn TTF_RenderUTF8_Blended(font: *const TTF_Font, text: *const c_char, fg: SDL_Color)                -> *mut SDL_Surface;
  }
}

/// SDL Font datatype.
pub struct Font {
  p: *mut ffi::TTF_Font
}

fn ensure_init() {
  unsafe {
    if ffi::TTF_WasInit() == 0 {
      assert_eq!(ffi::TTF_Init(), 0);
    }
  }
}

impl Font {
  #[allow(missing_docs)]
  pub fn new(font: &Path, point_size: u32) -> Font {
    ensure_init();

    let c_path = CString::new(font.to_str().unwrap().as_bytes()).unwrap();
    let ptr = c_path.as_ptr() as *const i8;
    let p = unsafe { ffi::TTF_OpenFont(ptr, point_size as ffi::c_int) };

    assert!(!p.is_null());

    Font { p: p }
  }

  /// Color is rgba
  pub fn render<'a, 'b:'a>(
    &self,
    gl: &'a GLContext,
    txt: &str,
    color: Color4<u8>,
  ) -> Texture2D<'b> {
    let sdl_color = SDL_Color {
      r: color.r,
      g: color.g,
      b: color.b,
      a: color.a
    };

    let surface_ptr = {
      let c_str = CString::new(txt.as_bytes()).unwrap();
      let ptr = c_str.as_ptr() as *const i8;
      unsafe {
        ffi::TTF_RenderUTF8_Blended(self.p as *const ffi::c_void, ptr, sdl_color)
      }
    };

    let tex = unsafe {
      surface_ptr.as_ref().expect("Cannot render text.")
    };

    unsafe {
      assert_eq!((*tex.format).format, SDL_PIXELFORMAT_ARGB8888);
    }

    let texture = Texture2D::new(gl);
    unsafe {
      gl::BindTexture(gl::TEXTURE_2D, texture.handle.gl_id);
      gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, tex.w, tex.h, 0, gl::BGRA, gl::UNSIGNED_INT_8_8_8_8_REV, tex.pixels);
      gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
      gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

      surface::SDL_FreeSurface(surface_ptr as *const SDL_Surface);
    }

    texture
  }

  /// Color the text red.
  #[allow(dead_code)]
  pub fn red<'a>(&self, gl: &'a GLContext, txt: &str) -> Texture2D<'a> {
    self.render(gl, txt, Color4::of_rgba(0xFF, 0x00, 0x00, 0xFF))
  }

  /// dark black #333
  #[allow(dead_code)]
  pub fn dark<'a>(&self, gl: &'a GLContext, txt: &str) -> Texture2D<'a> {
    self.render(gl, txt, Color4::of_rgba(0x33, 0x33, 0x33, 0xFF))
  }

  /// light black #555
  #[allow(dead_code)]
  pub fn light<'a>(&self, gl: &'a GLContext, txt: &str) -> Texture2D<'a> {
    self.render(gl, txt, Color4::of_rgba(0x55, 0x55, 0x55, 0xFF))
  }
}

impl Drop for Font {
  fn drop(&mut self) {
    unsafe { ffi::TTF_CloseFont(self.p) }
  }
}

#[test]
fn load_and_unload() {
  Font::new(&Path::new("fonts/Open_Sans/OpenSans-Regular.ttf"), 12);
}
