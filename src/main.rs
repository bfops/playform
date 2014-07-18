#![feature(globs)] // Allow global imports

extern crate cgmath;
extern crate gl;
extern crate graphics;
extern crate native;
extern crate piston;
extern crate sdl2_game_window;

use sdl2_game_window::GameWindowSDL2;

use cgmath::array::*;
use cgmath::vector::{Vector3, Vector4};
use piston::*;
use gl::types::*;
use std::mem;
use std::ptr;
use std::str;

pub struct Color4<T> { inner: Vector4<T>, }

impl<T: Clone> Clone for Color4<T> {
  fn clone(&self) -> Color4<T> {
    Color4 { inner: self.inner.clone(), }
  }
}

impl<T: Clone> Color4<T> {
  fn new(v: &Vector4<T>) -> Color4<T> {
    Color4 { inner: v.clone(), }
  }
}

pub struct Vertex<T> {
  position: Vector3<T>,
  color: Color4<T>,
}

impl<T: Clone> Clone for Vertex<T> {
  fn clone(&self) -> Vertex<T> {
    Vertex {
      position: self.position.clone(),
      color: self.color.clone(),
    }
  }
}

impl<T: Clone> Vertex<T> {
  fn new(v: &Vector3<T>, c: &Color4<T>) -> Vertex<T> {
    Vertex {
      position: v.clone(),
      color: c.clone(),
    }
  }
}

pub struct Triangle<T> {
  vertices: [Vertex<T>, ..3],
}

impl<T: Clone> Clone for Triangle<T> {
  fn clone(&self) -> Triangle<T> {
    Triangle {
      vertices: [self.vertices[0].clone(), self.vertices[1].clone(), self.vertices[2].clone()],
    }
  }
}

pub struct App {
  vao: u32,
  vbo: u32,
  vertex_data: Vec<Triangle<GLfloat>>,
  projection_matrix: cgmath::matrix::Matrix4<GLfloat>,
  shader_program: u32,
}

pub fn translate(t: &Vector3<GLfloat>) -> cgmath::matrix::Matrix4<GLfloat> {
  cgmath::matrix::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    t.x, t.y, t.z, 1.0,
  )
}

impl<T: Clone> Triangle<T> {
  fn new(
      v1: &Vertex<T>,
      v2: &Vertex<T>,
      v3: &Vertex<T>)
      -> Triangle<T> {
    Triangle {
      vertices: [v1.clone(), v2.clone(), v3.clone()],
    }
  }
}

impl Game for App {
  fn key_press(&mut self, _args: &KeyPressArgs) {
    match _args.key {
      piston::keyboard::A => {
        self.transform_projection(&translate(&Vector3::new(0.1, 0.0, 0.0)));
      },
      piston::keyboard::D => {
        self.transform_projection(&translate(&Vector3::new(-0.1, 0.0, 0.0)));
      },
      piston::keyboard::LShift => {
        self.transform_projection(&translate(&Vector3::new(0.0, 0.1, 0.0)));
      },
      piston::keyboard::Space => {
        self.transform_projection(&translate(&Vector3::new(0.0, -0.1, 0.0)));
      },
      piston::keyboard::W => {
        self.transform_projection(&translate(&Vector3::new(0.0, 0.0, 0.1)));
      },
      piston::keyboard::S => {
        self.transform_projection(&translate(&Vector3::new(0.0, 0.0, -0.1)));
      },
      _ => {},
    }
  }

  fn load(&mut self) {
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);

    self.shader_program = link_program(vs, fs);

    unsafe {
      // Create Vertex Array Object
      gl::GenVertexArrays(1, &mut self.vao);
      gl::BindVertexArray(self.vao);

      let raw_data = mem::transmute(&self.vertex_data.slice(0, self.vertex_data.len())[0]);

      // Create a Vertex Buffer Object and copy the vertex data to it
      gl::GenBuffers(1, &mut self.vbo);
      gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
      gl::BufferData(gl::ARRAY_BUFFER,
                      (self.vertex_data.len() * mem::size_of::<Triangle<GLfloat>>()) as GLsizeiptr,
                      raw_data,
                      gl::STATIC_DRAW);

      gl::UseProgram(self.shader_program);
      let loc = gl::GetUniformLocation(self.shader_program, "proj_matrix".to_c_str().unwrap());
      if loc == -1 {
        println!("couldn't read matrix");
      } else {
        self.projection_matrix = cgmath::projection::frustum::<GLfloat>(-1.0, 1.0, -1.0, 1.0, 0.1, 2.0);
        self.transform_projection(&translate(&Vector3::new(0.0, 0.0, -0.1)));
        gl::UniformMatrix4fv(loc, 1, 0, mem::transmute(self.projection_matrix.ptr()));
      }
      "out_color".with_c_str(|ptr| gl::BindFragDataLocation(self.shader_program, 0, ptr));

      // Specify the layout of the vertex data
      let pos_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(self.shader_program, ptr));
      gl::EnableVertexAttribArray(pos_attr as GLuint);
      let color_attr = "in_color".with_c_str(|ptr| gl::GetAttribLocation(self.shader_program, ptr));
      gl::EnableVertexAttribArray(color_attr as GLuint);

      gl::VertexAttribPointer(
          pos_attr as GLuint,
          (mem::size_of::<Vector3<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<Vertex<GLfloat>>() as i32,
          ptr::null(),
      );
      gl::VertexAttribPointer(
          color_attr as GLuint,
          (mem::size_of::<Color4<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<Vertex<GLfloat>>() as i32,
          ptr::null().offset(mem::size_of::<Vector3<GLfloat>>() as int),
      );
    }

    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
  }

  fn render(&mut self, _:&RenderArgs) {
    gl::Clear(gl::COLOR_BUFFER_BIT);

    // Draw a triangle from the 3 vertices
    gl::DrawArrays(gl::TRIANGLES, 0, 3);
  }
}

impl App {
  pub fn new() -> App {
    App {
      vao: 0,
      vbo: 0,
      vertex_data: Vec::from_slice([
        Triangle::new(
          &Vertex::new(&Vector3::new( 0.0,  0.5, 0.0), &Color4::new(&Vector4::new(1.0, 0.0, 0.0, 1.0))),
          &Vertex::new(&Vector3::new( 0.5, -0.5, 0.0), &Color4::new(&Vector4::new(0.0, 1.0, 0.0, 1.0))),
          &Vertex::new(&Vector3::new(-0.5, -0.5, 0.0), &Color4::new(&Vector4::new(0.0, 0.0, 1.0, 1.0))),
        ),
      ]),
      shader_program: -1 as u32,
      projection_matrix: cgmath::matrix::Matrix4::from_value(0.0),
    }
  }

  pub fn transform_projection(&mut self, t: &cgmath::matrix::Matrix4<GLfloat>) {
    unsafe {
      let loc = gl::GetUniformLocation(self.shader_program, "proj_matrix".to_c_str().unwrap());
      if loc == -1 {
        println!("couldn't read matrix");
      } else {
        self.projection_matrix = self.projection_matrix * *t;
        gl::UniformMatrix4fv(loc, 1, 0, mem::transmute(self.projection_matrix.ptr()));
      }
    }
  }

  pub fn destroy(&mut self) {
    unsafe {
      gl::DeleteBuffers(1, &self.vbo);
      gl::DeleteVertexArrays(1, &self.vao);
    }
  }
}

// Shader sources
static VS_SRC: &'static str =
   "#version 150\n\
uniform mat4 proj_matrix;\n\
in vec3 position;\n\
in vec4 in_color;\n\
varying vec4 color;\n\
void main() {\n\
gl_Position = proj_matrix * vec4(position, 1.0);\n\
color = in_color;\n\
}";

static FS_SRC: &'static str =
   "#version 150\n\
varying vec4 color;\n\
out vec4 out_color;\n\
void main() {\n\
out_color = color;\n\
}";

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader = gl::CreateShader(ty);
    unsafe {
        // Attempt to compile the shader
        src.with_c_str(|ptr| gl::ShaderSource(shader, 1, &ptr, ptr::null()));
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::from_elem(len as uint - 1, 0u8); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            fail!("{}", str::from_utf8(buf.as_slice()).expect("ShaderInfoLog not valid utf8"));
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    let program = gl::CreateProgram();

    gl::AttachShader(program, vs);
    gl::AttachShader(program, fs);
    gl::LinkProgram(program);

    unsafe {
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::from_elem(len as uint - 1, 0u8); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            fail!("{}", str::from_utf8(buf.as_slice()).expect("ProgramInfoLog not valid utf8"));
        }
    }

    program
}

fn main() {
  let mut window = GameWindowSDL2::new(
    GameWindowSettings {
      title: "playform".to_string(),
      size: [800, 600],
      fullscreen: false,
      exit_on_esc: false,
    }
  );

  App::new().run(&mut window, &GameIteratorSettings {
    updates_per_second: 30,
    max_frames_per_second: 30,
  });
}
