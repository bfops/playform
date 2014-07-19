#![feature(globs)] // Allow global imports

extern crate cgmath;
extern crate gl;
extern crate graphics;
extern crate native;
extern crate piston;
extern crate sdl2_game_window;

use sdl2_game_window::GameWindowSDL2;

use cgmath::array::*;
use cgmath::*;
use piston::*;
use gl::types::*;
use std::vec::*;
use std::*;

pub struct Color4<T> { r: T, g: T, b: T, a: T }

impl<T: Clone> Clone for Color4<T> {
  fn clone(&self) -> Color4<T> {
    Color4 {
      r: self.r.clone(),
      g: self.g.clone(),
      b: self.b.clone(),
      a: self.a.clone(),
    }
  }
}

impl<T: Clone> Color4<T> {
  fn new(r: &T, g: &T, b: &T, a: &T) -> Color4<T> {
    Color4 {
      r: r.clone(),
      g: g.clone(),
      b: b.clone(),
      a: a.clone(),
    }
  }
}

pub struct Vertex<T> {
  position: vector::Vector3<T>,
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
  fn new(x: &T, y: &T, z: &T, c: &Color4<T>) -> Vertex<T> {
    Vertex {
      position: vector::Vector3::new(x.clone(), y.clone(), z.clone()),
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

#[deriving(Clone)]
pub enum BlockType {
  Grass,
  Dirt,
  Stone,
}

impl BlockType {
  pub fn to_color(&self) -> Color4<GLfloat> {
    match *self {
      Grass => Color4::new(&0.0, &0.5,  &0.0, &1.0),
      Dirt  => Color4::new(&0.2, &0.15, &0.1, &1.0),
      Stone => Color4::new(&0.5, &0.5,  &0.5, &1.0),
    }
  }
}

#[deriving(Clone)]
pub struct Block {
  low_corner: vector::Vector3<GLfloat>,
  high_corner: vector::Vector3<GLfloat>,
  block_type: BlockType,
}

impl Block {
  fn new(low_corner: &vector::Vector3<GLfloat>, high_corner: &vector::Vector3<GLfloat>, block_type: BlockType) -> Block {
    Block {
      low_corner: low_corner.clone(),
      high_corner: high_corner.clone(),
      block_type: block_type,
    }
  }

  fn to_triangles(&self) -> [Triangle<GLfloat>, ..12] {
    let (x1, y1, z1) = (self.low_corner.x, self.low_corner.y, self.low_corner.z);
    let (x2, y2, z2) = (self.high_corner.x, self.high_corner.y, self.high_corner.z);
    let c = self.block_type.to_color();
    [
      // front
      Triangle::new(&Vertex::new(&x1, &y1, &z1, &c), &Vertex::new(&x1, &y2, &z1, &c), &Vertex::new(&x2, &y2, &z1, &c)),
      Triangle::new(&Vertex::new(&x1, &y1, &z1, &c), &Vertex::new(&x2, &y2, &z1, &c), &Vertex::new(&x2, &y1, &z1, &c)),
      // left
      Triangle::new(&Vertex::new(&x1, &y1, &z2, &c), &Vertex::new(&x1, &y2, &z2, &c), &Vertex::new(&x1, &y2, &z1, &c)),
      Triangle::new(&Vertex::new(&x1, &y1, &z2, &c), &Vertex::new(&x1, &y2, &z1, &c), &Vertex::new(&x1, &y1, &z1, &c)),
      // top
      Triangle::new(&Vertex::new(&x1, &y2, &z1, &c), &Vertex::new(&x1, &y2, &z2, &c), &Vertex::new(&x2, &y2, &z2, &c)),
      Triangle::new(&Vertex::new(&x1, &y2, &z1, &c), &Vertex::new(&x2, &y2, &z2, &c), &Vertex::new(&x2, &y2, &z1, &c)),
      // back
      Triangle::new(&Vertex::new(&x2, &y1, &z2, &c), &Vertex::new(&x2, &y2, &z2, &c), &Vertex::new(&x1, &y2, &z2, &c)),
      Triangle::new(&Vertex::new(&x2, &y1, &z2, &c), &Vertex::new(&x1, &y2, &z2, &c), &Vertex::new(&x1, &y1, &z2, &c)),
      // right
      Triangle::new(&Vertex::new(&x2, &y1, &z1, &c), &Vertex::new(&x2, &y2, &z1, &c), &Vertex::new(&x2, &y2, &z2, &c)),
      Triangle::new(&Vertex::new(&x2, &y1, &z1, &c), &Vertex::new(&x2, &y2, &z2, &c), &Vertex::new(&x2, &y1, &z2, &c)),
      // bottom
      Triangle::new(&Vertex::new(&x1, &y1, &z2, &c), &Vertex::new(&x1, &y1, &z1, &c), &Vertex::new(&x2, &y1, &z1, &c)),
      Triangle::new(&Vertex::new(&x1, &y1, &z2, &c), &Vertex::new(&x2, &y1, &z1, &c), &Vertex::new(&x2, &y1, &z2, &c)),
    ]
  }
}

pub struct App {
  vao: u32,
  vbo: u32,
  triangles: Vec<Triangle<GLfloat>>,
  world_data: Vec<Block>,
  projection_matrix: cgmath::matrix::Matrix4<GLfloat>,
  shader_program: u32,
}

pub fn translate(t: &vector::Vector3<GLfloat>) -> cgmath::matrix::Matrix4<GLfloat> {
  cgmath::matrix::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    t.x, t.y, t.z, 1.0,
  )
}

pub fn perspective(fovy: GLfloat, aspect: GLfloat, near: GLfloat, far: GLfloat) -> cgmath::matrix::Matrix4<GLfloat> {
  cgmath::matrix::Matrix4::new(
    fovy / aspect, 0.0, 0.0,                              0.0,
    0.0,          fovy, 0.0,                              0.0,
    0.0,           0.0, (near + far) / (near - far),     -1.0,
    0.0,           0.0, 2.0 * near * far / (near - far),  0.0,
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
        self.transform_projection(&translate(&vector::Vector3::new(0.1, 0.0, 0.0)));
      },
      piston::keyboard::D => {
        self.transform_projection(&translate(&vector::Vector3::new(-0.1, 0.0, 0.0)));
      },
      piston::keyboard::LShift => {
        self.transform_projection(&translate(&vector::Vector3::new(0.0, 0.1, 0.0)));
      },
      piston::keyboard::Space => {
        self.transform_projection(&translate(&vector::Vector3::new(0.0, -0.1, 0.0)));
      },
      piston::keyboard::W => {
        self.transform_projection(&translate(&vector::Vector3::new(0.0, 0.0, 0.1)));
      },
      piston::keyboard::S => {
        self.transform_projection(&translate(&vector::Vector3::new(0.0, 0.0, -0.1)));
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

      // Create a Vertex Buffer Object and copy the vertex data to it
      gl::GenBuffers(1, &mut self.vbo);
      gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
      gl::BufferData(gl::ARRAY_BUFFER,
                      (self.triangles.len() * mem::size_of::<Triangle<GLfloat>>()) as GLsizeiptr,
                      mem::transmute(self.triangles.get(0)),
                      gl::STATIC_DRAW);

      gl::UseProgram(self.shader_program);
      let loc = gl::GetUniformLocation(self.shader_program, "proj_matrix".to_c_str().unwrap());
      if loc == -1 {
        println!("couldn't read matrix");
      } else {
        self.projection_matrix = perspective(3.14/2.0, 4.0/3.0, 0.1, 100.0);
        self.transform_projection(&translate(&vector::Vector3::new(0.0, -2.0, -12.0)));
      }
      "out_color".with_c_str(|ptr| gl::BindFragDataLocation(self.shader_program, 0, ptr));

      // Specify the layout of the vertex data
      let pos_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(self.shader_program, ptr));
      gl::EnableVertexAttribArray(pos_attr as GLuint);
      let color_attr = "in_color".with_c_str(|ptr| gl::GetAttribLocation(self.shader_program, ptr));
      gl::EnableVertexAttribArray(color_attr as GLuint);

      gl::VertexAttribPointer(
          pos_attr as GLuint,
          (mem::size_of::<vector::Vector3<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
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
          ptr::null().offset(mem::size_of::<vector::Vector3<GLfloat>>() as int),
      );
    }

    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
  }

  fn render(&mut self, _:&RenderArgs) {
    gl::Clear(gl::COLOR_BUFFER_BIT);

    gl::DrawArrays(gl::TRIANGLES, 0, 3 * self.triangles.len() as i32);
  }
}

impl App {
  pub fn new() -> App {
    let mut world_data = Vec::new();
    let mut i = -16i;
    while i <= 16 {
      let mut j = -16i;
      while j <= 16 {
        let (x1, y1, z1) = (i as GLfloat - 0.5, 0.0, j as GLfloat - 0.5);
        let (x2, y2, z2) = (i as GLfloat + 0.5, 1.0, j as GLfloat + 0.5);
        world_data.grow(1, &Block::new(&vector::Vector3::new(x1, y1, z1), &vector::Vector3::new(x2, y2, z2), Grass));
        j += 1;
      }
      i += 1;
    }
    // front wall
    i = -16i;
    while i <= 16 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (i as GLfloat - 0.5, 1.0 + j as GLfloat, -16.0 - 0.5);
        let (x2, y2, z2) = (i as GLfloat + 0.5, 2.0 + j as GLfloat, -16.0 + 0.5);
        world_data.grow(1, &Block::new(&vector::Vector3::new(x1, y1, z1), &vector::Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    // back wall
    i = -16i;
    while i <= 16 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (i as GLfloat - 0.5, 1.0 + j as GLfloat, 16.0 - 0.5);
        let (x2, y2, z2) = (i as GLfloat + 0.5, 2.0 + j as GLfloat, 16.0 + 0.5);
        world_data.grow(1, &Block::new(&vector::Vector3::new(x1, y1, z1), &vector::Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    // left wall
    i = -16i;
    while i <= 16 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (-16.0 - 0.5, 1.0 + j as GLfloat, i as GLfloat - 0.5);
        let (x2, y2, z2) = (-16.0 + 0.5, 2.0 + j as GLfloat, i as GLfloat + 0.5);
        world_data.grow(1, &Block::new(&vector::Vector3::new(x1, y1, z1), &vector::Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    // right wall
    i = -16i;
    while i <= 16 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (16.0 - 0.5, 1.0 + j as GLfloat, i as GLfloat - 0.5);
        let (x2, y2, z2) = (16.0 + 0.5, 2.0 + j as GLfloat, i as GLfloat + 0.5);
        world_data.grow(1, &Block::new(&vector::Vector3::new(x1, y1, z1), &vector::Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    let mut app = App {
      vao: 0,
      vbo: 0,
      triangles: Vec::new(),
      world_data: world_data,
      shader_program: -1 as u32,
      projection_matrix: cgmath::matrix::Matrix4::from_value(0.0),
    };

    app.update_triangles();
    app
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

  pub fn update_triangles(&mut self) {
    let mut i = 0;
    self.triangles = Vec::new();
    while i < self.world_data.len() {
      self.triangles.push_all(self.world_data.get(i).to_triangles());
      i += 1;
    }
  }

  pub fn drop(&self) {
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
