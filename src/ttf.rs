extern crate gl;

use color::Color4;
use sdl2::pixels::ll::SDL_Color;
use sdl2::surface::ll::SDL_Surface;
use sdl2::surface;

use std::path::Path;

#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub mod ffi {
  extern crate libc;

  use sdl2::pixels::ll::SDL_Color;
  use sdl2::surface::ll::SDL_Surface;

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
  pub fn new(font: &Path, point_size: uint) -> Font {
    ensure_init();

    let c_path = font.to_c_str();
    let p = unsafe { ffi::TTF_OpenFont(c_path.as_ptr(), point_size as ffi::c_int) };

    Font { p: p }
  }

  /// Color is rgba
  pub fn render(&self, txt: &str, color: Color4<u8>) {
    unsafe {
      let sdl_color = SDL_Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a
      };

      let surface_ptr = txt.with_c_str(|c_txt| {
          ffi::TTF_RenderUTF8_Blended(self.p as *const ffi::c_void, c_txt, sdl_color)
        });

      let tex = surface_ptr.to_option().expect("Cannot render text.");

      let mut texture = 0;
      gl::GenTextures(1, &mut texture);
      gl::BindTexture(gl::TEXTURE_2D, texture);
      gl::TexImage2D(gl::TEXTURE_2D, 0, 3, tex.w, tex.h, 0, gl::BGR, gl::UNSIGNED_BYTE, tex.pixels);
      gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
      gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

      surface::ll::SDL_FreeSurface(surface_ptr as *const SDL_Surface);
    }
  }

  pub fn red(&self, txt: &str) {
    self.render(txt, Color4::new(0xFF, 0x00, 0x00, 250))
  }

  /// dark black #333
  pub fn dark(&self, txt: &str) {
    self.render(txt, Color4::new(0x33, 0x33, 0x33, 250))
  }

  /// light black #555
  pub fn light(&self, txt: &str) {
    self.render(txt, Color4::new(0x55, 0x55, 0x55, 250))
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
