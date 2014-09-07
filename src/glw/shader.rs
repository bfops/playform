use camera::Camera;
use gl;
use gl::types::*;
use gl_context::GLContext;
use light::Light;
use nalgebra::na::{Vec3, Mat4};
use std::collections::HashMap;
use std::io::fs::File;
use std::mem;
use std::ptr;
use std::str;

pub struct Shader {
  pub id: GLuint,
  pub components: Vec<GLuint>,
  pub uniforms: HashMap<String, GLint>,
}

impl Shader {
  pub fn new(gl: &mut GLContext, shader_components: Vec<(String, GLenum)>) -> Shader {
    let program = gl::CreateProgram();

    let mut components = Vec::new();
    for (content, component) in shader_components.move_iter() {
      let s = gl.compile_shader(content, component);
      gl::AttachShader(program, s);
      components.push(s);
    }

    gl::LinkProgram(program);

    // Get the link status
    let mut status = gl::FALSE as GLint;
    unsafe {
      gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    }

    // Fail on error
    if status != (gl::TRUE as GLint) {
        let mut len: GLint = 0;
        unsafe {
          gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        }
        let mut buf = Vec::from_elem(len as uint - 1, 0u8); // subtract 1 to skip the trailing null character
        unsafe {
          gl::GetProgramInfoLog(program, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
        }
        fail!("{}", str::from_utf8(buf.as_slice()).expect("ProgramInfoLog not valid utf8"));
    }

    Shader {
      id: program,
      components: components,
      uniforms: HashMap::new(),
    }
  }

  pub fn from_files(
    gl: &mut GLContext,
    component_paths: &[(&str, GLenum)],
  ) -> Shader {
    let mut components = Vec::new();
    for &(path, component_type) in component_paths.iter() {
      match File::open(&Path::new(path)) {
        Ok(mut f) =>
          match f.read_to_string() {
            Ok(s) => {
              components.push((s, component_type));
            },
            Err(e) => {
              fail!("Couldn't read shader file \"{}\": {}", path, e);
            }
          },
        Err(e) => {
          fail!("Couldn't open shader file \"{}\" for reading: {}", path, e);
        }
      }
    }

    Shader::new(gl, components)
  }

  pub fn with_uniform_location<T>(
    &mut self,
    gl: &mut GLContext,
    name: &'static str,
    f: |GLint| -> T,
  ) -> T {
    let s_name = String::from_str(name);
    let name = gl.scache.convert(name).as_ptr();
    match self.uniforms.find(&s_name) {
      None => {
        let (loc, t) = gl.use_shader(self, |_| {
          let loc = unsafe { gl::GetUniformLocation(self.id, name) };
          assert!(loc != -1, "couldn't find shader uniform {}", name);

          match gl::GetError() {
            gl::NO_ERROR => {},
            err => fail!("OpenGL error 0x{:x} in GetUniformLocation", err),
          }

          (loc, f(loc))
        });

        self.uniforms.insert(s_name, loc);
        t
      },
      Some(&loc) => gl.use_shader(self, |_| f(loc)),
    }
  }

  /// Sets the variable `light` in some shader.
  pub fn set_point_light(&mut self, gl: &mut GLContext, light: &Light) {
    self.with_uniform_location(gl, "light.position", |light_pos| {
      gl::Uniform3f(light_pos, light.position.x, light.position.y, light.position.z);
    });
    self.with_uniform_location(gl, "light.intensity", |light_intensity| {
      gl::Uniform3f(light_intensity, light.intensity.x, light.intensity.y, light.intensity.z);
    });
  }

  pub fn set_ambient_light(&mut self, gl: &mut GLContext, intensity: Vec3<GLfloat>) {
    self.with_uniform_location(gl, "ambient_light", |loc| {
      gl::Uniform3f(loc, intensity.x, intensity.y, intensity.z);
    });
  }

  /// Sets the variable `projection_matrix` in some shader.
  pub fn set_projection_matrix(&mut self, gl: &mut GLContext, m: &Mat4<GLfloat>) {
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

  pub fn set_camera(&mut self, gl: &mut GLContext, c: &Camera) {
    self.set_projection_matrix(gl, &c.projection_matrix());
  }
}

impl Drop for Shader {
  fn drop(&mut self) {
    gl::DeleteProgram(self.id);
    for &s in self.components.iter() {
      gl::DeleteShader(s);
    }
  }
}
