use gl;
use gl::types::*;
use gl_buffer::GLBuffer;
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
  pub gl_id: GLuint,
}

impl Texture {
  pub fn bind_2d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_2D, self.gl_id);

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }
  }

  #[allow(dead_code)]
  pub fn bind_3d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_3D, self.gl_id);

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x}", err),
    }
  }
}

impl Drop for Texture {
  fn drop(&mut self) {
    unsafe { gl::DeleteTextures(1, &self.gl_id); }
  }
}

/// See the OpenGL docs on buffer textures.
pub struct BufferTexture<T> {
  pub texture: Texture,
  pub buffer: GLBuffer<T>,
}

impl<T> BufferTexture<T> {
  pub fn new(_gl: &GLContext, format: GLenum, capacity: uint) -> BufferTexture<T> {
    // TODO: enforce that `format` matches T.

    let buffer = GLBuffer::new(capacity);

    let mut gl_id = 0;
    unsafe {
      gl::GenTextures(1, &mut gl_id);
    }

    gl::BindTexture(gl::TEXTURE_BUFFER, gl_id);
    gl::TexBuffer(gl::TEXTURE_BUFFER, format, buffer.byte_buffer.gl_id);

    BufferTexture {
      texture: Texture { gl_id: gl_id },
      buffer: buffer,
    }
  }
}
