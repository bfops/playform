use camera::Camera;
use gl;
use gl::types::*;
use gl_context::GLContext;
use nalgebra::na::Mat4;
use std::mem;

pub struct Shader {
  pub id: GLuint,
  pub vs: GLuint,
  pub fs: GLuint,
}

impl Shader {
  pub fn new(gl: &mut GLContext, vertex_shader: &str, fragment_shader: &str) -> Shader {
    let vs = gl.compile_shader(vertex_shader, gl::VERTEX_SHADER);
    let fs = gl.compile_shader(fragment_shader, gl::FRAGMENT_SHADER);
    let id = gl.link_shader(vs, fs);
    Shader { id: id, vs: vs, fs: fs }
  }

  /// Sets the variable `projection_matrix` in some shader.
  pub fn set_projection_matrix(&self, gl: &mut GLContext, m: &Mat4<GLfloat>) {
    let var_name = gl.scache.convert("projection_matrix").as_ptr();
    gl.use_shader(self, |_gl| {
      unsafe {
        let loc = gl::GetUniformLocation(self.id, var_name);
        assert!(loc != -1, "couldn't read projection_matrix");

        match gl::GetError() {
          gl::NO_ERROR => {},
          err => fail!("OpenGL error 0x{:x} in GetUniformLocation", err),
        }

        let p = mem::transmute(m);
        gl::UniformMatrix4fv(loc, 1, 0, p);

        match gl::GetError() {
          gl::NO_ERROR => {},
          err => fail!("OpenGL error 0x{:x} in UniformMat4fv", err),
        }
      }
    })
  }

  pub fn set_camera(&self, gl: &mut GLContext, c: &Camera) {
    self.set_projection_matrix(gl, &c.projection_matrix());
  }
}

impl Drop for Shader {
  fn drop(&mut self) {
    gl::DeleteProgram(self.id);
    gl::DeleteShader(self.vs);
    gl::DeleteShader(self.fs);
  }
}
