use gl;
use gl::types::*;
use gl_context::GLContext;

// TODO(cgaebel): Handle texture creation from an SDL surface.

/// A GPU-allocated texture.
pub struct Texture {
  pub id: GLuint,
}

impl Texture {
  pub fn bind_2d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_2D, self.id);
  }

  #[allow(dead_code)]
  pub fn bind_3d(&self, _gl: &GLContext) {
    gl::BindTexture(gl::TEXTURE_3D, self.id);
  }
}

impl Drop for Texture {
  fn drop(&mut self) {
    unsafe { gl::DeleteTextures(1, &self.id); }
  }
}
