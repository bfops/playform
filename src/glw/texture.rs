use gl;
use gl::types::*;
use gl_context::GLContext;
use std::default::Default;

// TODO(cgaebel): Handle texture creation from an SDL surface.

#[deriving(Clone)]
pub struct TextureUnit {
  pub glsl_id: GLuint,
}

impl TextureUnit {
  // TODO: consider making this part of the struct to avoid recalculating;
  // is that fetch cheaper than the addition of a constant?
  pub fn gl_id(&self) -> GLuint {
    gl::TEXTURE0 + self.glsl_id
  }
}

impl Default for TextureUnit {
  fn default() -> TextureUnit {
    TextureUnit {
      glsl_id: 0,
    }
  }
}

impl Add<u32, TextureUnit> for TextureUnit {
  fn add(&self, rhs: &u32) -> TextureUnit {
    TextureUnit {
      glsl_id: self.glsl_id + *rhs,
    }
  }
}

/// A GPU-allocated texture.
pub struct Texture {
  pub id: GLuint,
}

impl Texture {
  pub fn bind_2d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_2D, self.id);

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }
  }

  #[allow(dead_code)]
  pub fn bind_3d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_3D, self.id);

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }
  }
}

impl Drop for Texture {
  fn drop(&mut self) {
    unsafe { gl::DeleteTextures(1, &self.id); }
  }
}
