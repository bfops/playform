use camera::Camera;
use gl;
use gl::types::*;
use gl_context::GLContext;
use light::Light;
use nalgebra::na::{Vec3, Mat4};
use std::io::fs::File;
use std::mem;

pub struct Shader {
  pub id: GLuint,
  pub vs: GLuint,
  pub fs: GLuint,
}

impl Shader {
  pub fn new(gl: &mut GLContext, vertex_shader: String, fragment_shader: String) -> Shader {
    let vs = gl.compile_shader(vertex_shader, gl::VERTEX_SHADER);
    let fs = gl.compile_shader(fragment_shader, gl::FRAGMENT_SHADER);
    let id = gl.link_shader(vs, fs);
    Shader { id: id, vs: vs, fs: fs }
  }

  pub fn from_files(
    gl: &mut GLContext,
    vertex_shader_path: &str,
    fragment_shader_path: &str,
  ) -> Shader {
    match (File::open(&Path::new(vertex_shader_path)), File::open(&Path::new(fragment_shader_path))) {
      (Ok(mut vs), Ok(mut fs)) => {
        match (vs.read_to_string(), fs.read_to_string()) {
          (Ok(vs), Ok(fs)) => {
            Shader::new(gl, vs, fs)
          },
          _ =>
            fail!(
              "error reading shader files: \"{}\", \"{}\"",
              vertex_shader_path,
              fragment_shader_path
            ),
        }
      },
      _ =>
        fail!(
          "Couldn't open shader files for shader: \"{}\", \"{}\"",
          vertex_shader_path,
          fragment_shader_path
        ),
    }
  }

  fn with_uniform_location(&self, gl: &mut GLContext, name: &'static str, f: |GLint|) {
    let name = gl.scache.convert(name).as_ptr();
    gl.use_shader(self, |_| {
      unsafe {
        let loc = gl::GetUniformLocation(self.id, name);
        assert!(loc != -1, "couldn't find shader uniform {}", name);

        match gl::GetError() {
          gl::NO_ERROR => {},
          err => fail!("OpenGL error 0x{:x} in GetUniformLocation", err),
        }

        f(loc);
      }
    })
  }

  // TODO: these functions should take a &mut self.

  /// Sets the variable `projection_matrix` in some shader.
  pub fn set_projection_matrix(&self, gl: &mut GLContext, m: &Mat4<GLfloat>) {
    self.with_uniform_location(gl, "projection_matrix", |loc| {
      unsafe {
        let p = mem::transmute(m);
        gl::UniformMatrix4fv(loc, 1, 0, p);
      }

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in UniformMat4fv", err),
      }
    })
  }

  /// Sets the variable `light` in some shader.
  pub fn set_point_light(&self, gl: &mut GLContext, light: &Light) {
    self.with_uniform_location(gl, "light.position", |light_pos| {
      gl::Uniform3f(light_pos, light.position.x, light.position.y, light.position.z);
    });
    self.with_uniform_location(gl, "light.intensity", |light_intensity| {
      gl::Uniform3f(light_intensity, light.intensity.x, light.intensity.y, light.intensity.z);
    });
  }

  pub fn set_ambient_light(&self, gl: &mut GLContext, intensity: Vec3<GLfloat>) {
    self.with_uniform_location(gl, "ambient_light", |loc| {
      gl::Uniform3f(loc, intensity.x, intensity.y, intensity.z);
    });
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
