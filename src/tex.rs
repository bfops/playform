use cgmath;
use cgmath::obb;
use cgmath::vector::*;
use gl;
use gl::types::*;
use sdl2::surface::ll::SDL_Surface;

#[deriving(Clone)]
pub struct TextureVertex {
  pub position:         Vector2<GLfloat>,
  pub texture_position: Vector2<GLfloat>,
}

impl TextureVertex {
  #[inline]
  pub fn new(x: GLfloat, y: GLfloat, tx: GLfloat, ty: GLfloat) -> TextureVertex {
    TextureVertex {
      position: Vector2::new(x, y),
      texture_position: Vector2::new(tx, ty),
    }
  }
}

pub struct Texture {
  handle: GLuint,
  size:   Vector2<uint>,

  // If we've already calculated a preferred size, this keeps us from having
  // to constantly recompute the screen space size every frame, even when the
  // screen size hasn't actually changed.
  cached_preferred_size: Option<(Vector2<uint>, Vector2<GLfloat>)>,
}

fn floatify(x: Vector2<uint>) -> Vector2<GLfloat> {
  Vector2::new(x.x as GLfloat, x.y as GLfloat)
}

impl Texture {
  /// Create a texture with an explicit size, in pixels. Note that this still
  /// needs to be translated into screen space: [-1.0, 1.0].
  fn new(size: Vector2<uint>) -> Texture {
    let mut texture = -1;
    gl::GenTexture(&mut texture);
    assert!(texture != -1);
    gl::BindTexture(gl::TEXTURE_2D, texture);

    Texture { handle: texture, size: size }
  }

  pub fn of_surface(image: *const SDL_Surface) -> Texture {
    let ret = Texture::new(Vector2::new(image.w, image.h));

    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, image.w, image.h, 0, gl::BGRA, gl::UNSIGNED_INT_8_8_8_8_REV, image.pixels);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

    ret
  }

  /// Get the texture's preferred size, scaled to screen space.
  /// Feel free to translate this as necessary when rendering.
  pub fn preferred_size(&mut self, window_size: Vector2<uint>) -> Vector2<GLfloat> {
    match self.cached_preferred_size {
      None => {
        let ret = floatify(self.size).div_v(&floatify(window_size));
        self.cached_preferred_size = Some(window_size, ret);
        ret
      },
      Some((orig_window, scaled_resolution)) => {
        if orig_window == window_size { return scaled_resolution; }

        let ret = flatify(self.size).div_v(&floatify(window_size));
        self.cached_preferred_size = Some(window_size, ret);
        ret
      }
    }
  }
}

impl Drop for Texture {
  fn drop(&mut self) {
    gl::DeleteTexture(handle);
  }
}
