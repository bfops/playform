#![feature(globs)] // Allow global imports

extern crate core;
extern crate cgmath;
extern crate gl;
extern crate graphics;
extern crate native;
extern crate piston;
extern crate sdl2_game_window;

use sdl2_game_window::GameWindowSDL2;

use cgmath::*;
use cgmath::array::*;
use cgmath::matrix::*;
use cgmath::num::{BaseFloat};
use cgmath::vector::{Vector,Vector3};
use piston::*;
use gl::types::*;
use std::mem;
use std::ptr;
use std::str;
use std::num::*;
use std::vec::*;

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

// Rendering vertex: position and color.
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
  fn new(x: &T, y: &T, z: &T, c: &Color4<T>) -> Vertex<T> {
    Vertex {
      position: Vector3::new(x.clone(), y.clone(), z.clone()),
      color: c.clone(),
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
  // bounds of the Block
  low_corner: Vector3<GLfloat>,
  high_corner: Vector3<GLfloat>,
  block_type: BlockType,
}

enum Intersect {
  Intersect(Vector3<GLfloat>),
  NoIntersect,
}

enum Intersect1 {
  Within,
  Partial,
  NoIntersect1,
}

// Find whether two Blocks intersect.
fn intersect(b1: &Block, b2: &Block) -> Intersect {
  fn intersect1(x1l: GLfloat, x1h: GLfloat, x2l: GLfloat, x2h: GLfloat) -> Intersect1 {
    if x1l > x2l && x1h <= x2h {
      Within
    } else if x1h > x2l && x2h > x1l {
      Partial
    } else {
      NoIntersect1
    }
  }

  let mut ret = true;
  let mut v = Vector3::ident();
  match intersect1(b1.low_corner.x, b1.high_corner.x, b2.low_corner.x, b2.high_corner.x) {
    Within => { },
    Partial => { v.x = 0.0; },
    NoIntersect1 => { ret = false; },
  }
  match intersect1(b1.low_corner.y, b1.high_corner.y, b2.low_corner.y, b2.high_corner.y) {
    Within => { },
    Partial => { v.y = 0.0; },
    NoIntersect1 => { ret = false; },
  }
  match intersect1(b1.low_corner.z, b1.high_corner.z, b2.low_corner.z, b2.high_corner.z) {
    Within => { },
    Partial => { v.z = 0.0; },
    NoIntersect1 => { ret = false; },
  }

  if ret {
    Intersect(v)
  } else {
    NoIntersect
  }
}

impl Block {
  fn new(low_corner: &Vector3<GLfloat>, high_corner: &Vector3<GLfloat>, block_type: BlockType) -> Block {
    Block {
      low_corner: low_corner.clone(),
      high_corner: high_corner.clone(),
      block_type: block_type,
    }
  }

  // Construct the faces of the block as triangles for rendering.
  // Triangle vertices are in clockwise order when viewed from the outside of
  // the cube, for rendering purposes.
  fn to_triangles(&self) -> [Vertex<GLfloat>, ..36] {
    let (x1, y1, z1) = (self.low_corner.x, self.low_corner.y, self.low_corner.z);
    let (x2, y2, z2) = (self.high_corner.x, self.high_corner.y, self.high_corner.z);
    let c = self.block_type.to_color();
    [
      // front
      Vertex::new(&x1, &y1, &z1, &c), Vertex::new(&x1, &y2, &z1, &c), Vertex::new(&x2, &y2, &z1, &c),
      Vertex::new(&x1, &y1, &z1, &c), Vertex::new(&x2, &y2, &z1, &c), Vertex::new(&x2, &y1, &z1, &c),
      // left
      Vertex::new(&x1, &y1, &z2, &c), Vertex::new(&x1, &y2, &z2, &c), Vertex::new(&x1, &y2, &z1, &c),
      Vertex::new(&x1, &y1, &z2, &c), Vertex::new(&x1, &y2, &z1, &c), Vertex::new(&x1, &y1, &z1, &c),
      // top
      Vertex::new(&x1, &y2, &z1, &c), Vertex::new(&x1, &y2, &z2, &c), Vertex::new(&x2, &y2, &z2, &c),
      Vertex::new(&x1, &y2, &z1, &c), Vertex::new(&x2, &y2, &z2, &c), Vertex::new(&x2, &y2, &z1, &c),
      // back
      Vertex::new(&x2, &y1, &z2, &c), Vertex::new(&x2, &y2, &z2, &c), Vertex::new(&x1, &y2, &z2, &c),
      Vertex::new(&x2, &y1, &z2, &c), Vertex::new(&x1, &y2, &z2, &c), Vertex::new(&x1, &y1, &z2, &c),
      // right
      Vertex::new(&x2, &y1, &z1, &c), Vertex::new(&x2, &y2, &z1, &c), Vertex::new(&x2, &y2, &z2, &c),
      Vertex::new(&x2, &y1, &z1, &c), Vertex::new(&x2, &y2, &z2, &c), Vertex::new(&x2, &y1, &z2, &c),
      // bottom
      Vertex::new(&x1, &y1, &z2, &c), Vertex::new(&x1, &y1, &z1, &c), Vertex::new(&x2, &y1, &z1, &c),
      Vertex::new(&x1, &y1, &z2, &c), Vertex::new(&x2, &y1, &z1, &c), Vertex::new(&x2, &y1, &z2, &c),
    ]
  }

  // Construct outlines for this Block, to sharpen the edges.
  fn to_outlines(&self) -> [Vertex<GLfloat>, ..24] {
    let d = 0.002;
    let (x1, y1, z1) = (self.low_corner.x - d, self.low_corner.y - d, self.low_corner.z - d);
    let (x2, y2, z2) = (self.high_corner.x + d, self.high_corner.y + d, self.high_corner.z + d);
    let c = Color4::new(&0.0, &0.0, &0.0, &1.0);
    [
      Vertex::new(&x1, &y1, &z1, &c), Vertex::new(&x2, &y1, &z1, &c),
      Vertex::new(&x1, &y2, &z1, &c), Vertex::new(&x2, &y2, &z1, &c),
      Vertex::new(&x1, &y1, &z2, &c), Vertex::new(&x2, &y1, &z2, &c),
      Vertex::new(&x1, &y2, &z2, &c), Vertex::new(&x2, &y2, &z2, &c),

      Vertex::new(&x1, &y1, &z1, &c), Vertex::new(&x1, &y2, &z1, &c),
      Vertex::new(&x2, &y1, &z1, &c), Vertex::new(&x2, &y2, &z1, &c),
      Vertex::new(&x1, &y1, &z2, &c), Vertex::new(&x1, &y2, &z2, &c),
      Vertex::new(&x2, &y1, &z2, &c), Vertex::new(&x2, &y2, &z2, &c),

      Vertex::new(&x1, &y1, &z1, &c), Vertex::new(&x1, &y1, &z2, &c),
      Vertex::new(&x2, &y1, &z1, &c), Vertex::new(&x2, &y1, &z2, &c),
      Vertex::new(&x1, &y2, &z1, &c), Vertex::new(&x1, &y2, &z2, &c),
      Vertex::new(&x2, &y2, &z1, &c), Vertex::new(&x2, &y2, &z2, &c),
    ]
  }
}

pub struct App {
  world_data: Vec<Block>,
  // position; world coordinates
  camera_position: Vector3<GLfloat>,
  // speed; x/z units are relative to player facing
  camera_speed: Vector3<GLfloat>,
  // acceleration; x/z units are relative to player facing
  camera_accel: Vector3<GLfloat>,
  // renderable equivalent of world_data
  render_data: Vec<Vertex<GLfloat>>,
  triangles: uint, // number of triangles in render_data
  lines: uint, // number of lines in render_data
  // OpenGL projection matrix components
  fov_matrix: matrix::Matrix4<GLfloat>,
  translation_matrix: matrix::Matrix4<GLfloat>,
  rotation_matrix: matrix::Matrix4<GLfloat>,
  lateral_rotation: angle::Rad<GLfloat>,
  // OpenGL shader "program" id.
  shader_program: u32,
  // OpenGL vertex array id
  vao: u32,
  // OpenGL vertex buffer id
  vbo: u32,
}

// Create a 3D translation matrix.
pub fn translate(t: &Vector3<GLfloat>) -> matrix::Matrix4<GLfloat> {
  matrix::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    t.x, t.y, t.z, 1.0,
  )
}

// Create a 3D perspective initialization matrix.
pub fn perspective(fovy: GLfloat, aspect: GLfloat, near: GLfloat, far: GLfloat) -> matrix::Matrix4<GLfloat> {
  matrix::Matrix4::new(
    fovy / aspect, 0.0, 0.0,                              0.0,
    0.0,          fovy, 0.0,                              0.0,
    0.0,           0.0, (near + far) / (near - far),     -1.0,
    0.0,           0.0, 2.0 * near * far / (near - far),  0.0,
  )
}

/// Create a matrix from a rotation around an arbitrary axis
pub fn from_axis_angle<S: BaseFloat>(axis: &Vector3<S>, angle: angle::Rad<S>) -> Matrix4<S> {
    let (s, c) = angle::sin_cos(angle);
    let _1subc = one::<S>() - c;

    Matrix4::new(
        _1subc * axis.x * axis.x + c,
        _1subc * axis.x * axis.y + s * axis.z,
        _1subc * axis.x * axis.z - s * axis.y,
        zero(),

        _1subc * axis.x * axis.y - s * axis.z,
        _1subc * axis.y * axis.y + c,
        _1subc * axis.y * axis.z + s * axis.x,
        zero(),

        _1subc * axis.x * axis.z + s * axis.y,
        _1subc * axis.y * axis.z - s * axis.x,
        _1subc * axis.z * axis.z + c,
        zero(),

        zero(),
        zero(),
        zero(),
        one(),
    )
}

impl Game for App {
  fn key_press(&mut self, args: &KeyPressArgs) {
    match args.key {
      piston::keyboard::A => {
        self.walk(&-Vector3::unit_x());
      },
      piston::keyboard::D => {
        self.walk(&Vector3::unit_x());
      },
      piston::keyboard::LShift => {
        self.walk(&-Vector3::unit_y());
      },
      piston::keyboard::Space => {
        self.camera_accel.y = self.camera_accel.y + 0.4;
      },
      piston::keyboard::W => {
        self.walk(&-Vector3::unit_z());
      },
      piston::keyboard::S => {
        self.walk(&Vector3::unit_z());
      },
      piston::keyboard::Left => {
        let d = angle::rad(3.14 / 12.0 as GLfloat);
        self.lateral_rotation = self.lateral_rotation + d;
        self.rotate(&Vector3::unit_y(), d);
      },
      piston::keyboard::Right => {
        let d = angle::rad(-3.14 / 12.0 as GLfloat);
        self.lateral_rotation = self.lateral_rotation + d;
        self.rotate(&Vector3::unit_y(), d);
      },
      piston::keyboard::Up => {
        let axis = self.right();
        self.rotate(&axis, angle::rad(3.14/12.0 as GLfloat));
      },
      piston::keyboard::Down => {
        let axis = self.right();
        self.rotate(&axis, angle::rad(-3.14/12.0 as GLfloat));
      },
      _ => {},
    }
  }

  fn key_release(&mut self, args: &KeyReleaseArgs) {
    match args.key {
      // accelerations are negated from those in key_press.
      piston::keyboard::A => {
        self.walk(&Vector3::unit_x());
      },
      piston::keyboard::D => {
        self.walk(&-Vector3::unit_x());
      },
      piston::keyboard::LShift => {
        self.walk(&Vector3::unit_y());
      },
      piston::keyboard::Space => {
        self.camera_accel.y = self.camera_accel.y - 0.4;
      },
      piston::keyboard::W => {
        self.walk(&Vector3::unit_z());
      },
      piston::keyboard::S => {
        self.walk(&-Vector3::unit_z());
      },
      _ => { }
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

      // Create a Vertex Buffer Object.
      gl::GenBuffers(1, &mut self.vbo);
      gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

      // Set up shaders
      gl::UseProgram(self.shader_program);
      "out_color".with_c_str(|ptr| gl::BindFragDataLocation(self.shader_program, 0, ptr));

      let pos_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(self.shader_program, ptr));
      gl::EnableVertexAttribArray(pos_attr as GLuint);
      let color_attr = "in_color".with_c_str(|ptr| gl::GetAttribLocation(self.shader_program, ptr));
      gl::EnableVertexAttribArray(color_attr as GLuint);

      // Specify the layout of the vertex data:
      // position data first
      gl::VertexAttribPointer(
          pos_attr as GLuint,
          (mem::size_of::<Vector3<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<Vertex<GLfloat>>() as i32,
          ptr::null(),
      );
      // color data next
      gl::VertexAttribPointer(
          color_attr as GLuint,
          (mem::size_of::<Color4<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<Vertex<GLfloat>>() as i32,
          ptr::null().offset(mem::size_of::<Vector3<GLfloat>>() as int),
      );
    }

    gl::Enable(gl::LINE_SMOOTH);
    gl::LineWidth(2.5);

    gl::Enable(gl::DEPTH_TEST);
    gl::DepthFunc(gl::LESS);
    gl::ClearDepth(100.0);
    gl::ClearColor(0.0, 0.0, 0.0, 1.0);

    // initialize the projection matrix
    self.fov_matrix = perspective(3.14/2.0, 4.0/3.0, 0.1, 100.0);
    self.translate(&Vector3::new(0.0, 4.0, 10.0));
    self.update_projection();

    let mut i;
    // dirt block
    i = -1;
    while i <= 1 {
      let mut j = -1i;
      while j <= 1 {
        let (x1, y1, z1) = (3.0 + i as GLfloat, 6.0, 0.0 + j as GLfloat);
        let (x2, y2, z2) = (4.0 + i as GLfloat, 7.0, 1.0 + j as GLfloat);
        self.world_data.grow(1, &Block::new(&Vector3::new(x1, y1, z1), &Vector3::new(x2, y2, z2), Dirt));
        j += 1;
      }
      i += 1;
    }
    // ground
    i = -32i;
    while i <= 32 {
      let mut j = -32i;
      while j <= 32 {
        let (x1, y1, z1) = (i as GLfloat - 0.5, 0.0, j as GLfloat - 0.5);
        let (x2, y2, z2) = (i as GLfloat + 0.5, 1.0, j as GLfloat + 0.5);
        self.world_data.grow(1, &Block::new(&Vector3::new(x1, y1, z1), &Vector3::new(x2, y2, z2), Grass));
        j += 1;
      }
      i += 1;
    }
    // front wall
    i = -32i;
    while i <= 32 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (i as GLfloat - 0.5, 1.0 + j as GLfloat, -32.0 - 0.5);
        let (x2, y2, z2) = (i as GLfloat + 0.5, 2.0 + j as GLfloat, -32.0 + 0.5);
        self.world_data.grow(1, &Block::new(&Vector3::new(x1, y1, z1), &Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    // back wall
    i = -32i;
    while i <= 32 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (i as GLfloat - 0.5, 1.0 + j as GLfloat, 32.0 - 0.5);
        let (x2, y2, z2) = (i as GLfloat + 0.5, 2.0 + j as GLfloat, 32.0 + 0.5);
        self.world_data.grow(1, &Block::new(&Vector3::new(x1, y1, z1), &Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    // left wall
    i = -32i;
    while i <= 32 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (-32.0 - 0.5, 1.0 + j as GLfloat, i as GLfloat - 0.5);
        let (x2, y2, z2) = (-32.0 + 0.5, 2.0 + j as GLfloat, i as GLfloat + 0.5);
        self.world_data.grow(1, &Block::new(&Vector3::new(x1, y1, z1), &Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }
    // right wall
    i = -32i;
    while i <= 32 {
      let mut j = 0i;
      while j <= 32 {
        let (x1, y1, z1) = (32.0 - 0.5, 1.0 + j as GLfloat, i as GLfloat - 0.5);
        let (x2, y2, z2) = (32.0 + 0.5, 2.0 + j as GLfloat, i as GLfloat + 0.5);
        self.world_data.grow(1, &Block::new(&Vector3::new(x1, y1, z1), &Vector3::new(x2, y2, z2), Stone));
        j += 1;
      }
      i += 1;
    }

    self.update_render_data();
  }

  fn update(&mut self, _:&UpdateArgs) {
    let dP = Matrix3::from_axis_angle(&Vector3::unit_y(), self.lateral_rotation).mul_v(&self.camera_speed);
    if dP.x != 0.0 {
      self.translate(&Vector3::new(dP.x, 0.0, 0.0));
    }
    if dP.y != 0.0 {
      self.translate(&Vector3::new(0.0, dP.y, 0.0));
    }
    if dP.z != 0.0 {
      self.translate(&Vector3::new(0.0, 0.0, dP.z));
    }

    let dV = self.camera_accel;
    self.camera_speed = self.camera_speed + dV;
    // friction
    self.camera_speed = self.camera_speed.mul_s(0.8);
  }

  fn render(&mut self, _:&RenderArgs) {
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    gl::DrawArrays(gl::TRIANGLES, 0, self.triangles as i32);
    gl::DrawArrays(gl::LINES, self.triangles as GLint, self.lines as i32);
  }
}

impl App {
  pub fn new() -> App {
    App {
      world_data: Vec::new(),
      camera_position: Vector3::zero(),
      camera_speed: Vector3::zero(),
      camera_accel: Vector3::new(0.0, -0.15, 0.0),
      render_data: Vec::new(),
      triangles: 0,
      lines: 0,
      fov_matrix: Matrix4::identity(),
      translation_matrix: Matrix4::identity(),
      rotation_matrix: Matrix4::identity(),
      lateral_rotation: angle::rad(0.0),
      shader_program: -1 as u32,
      vao: 0,
      vbo: 0,
    }
  }

  // Update the OpenGL vertex data with the world data triangles.
  pub fn update_render_data(&mut self) {
    let mut triangles = Vec::new();
    let mut outlines = Vec::new();
    let mut i = 0;
    while i < self.world_data.len() {
      let block = self.world_data.get(i);
      triangles.push_all(block.to_triangles());
      outlines.push_all(block.to_outlines());
      i += 1;
    }

    self.triangles = triangles.len();
    self.lines = outlines.len();

    self.render_data = Vec::new();
    self.render_data.push_all(triangles.slice(0, triangles.len()));
    self.render_data.push_all(outlines.slice(0, outlines.len()));

    unsafe {
      gl::BufferData(
        gl::ARRAY_BUFFER,
        (self.render_data.len() * mem::size_of::<Vertex<GLfloat>>()) as GLsizeiptr,
        mem::transmute(self.render_data.get(0)),
        gl::STATIC_DRAW);
    }
  }

  pub fn update_projection(&self) {
    unsafe {
      let loc = gl::GetUniformLocation(self.shader_program, "proj_matrix".to_c_str().unwrap());
      if loc == -1 {
        println!("couldn't read matrix");
      } else {
        let projection = self.fov_matrix * self.rotation_matrix * self.translation_matrix;
        gl::UniformMatrix4fv(loc, 1, 0, mem::transmute(projection.ptr()));
      }
    }
  }

  #[inline]
  pub fn walk(&mut self, da: &Vector3<GLfloat>) {
    self.camera_accel = self.camera_accel + da.mul_s(0.2);
  }

  fn construct_player(&self, high_corner: &Vector3<GLfloat>) -> Block {
    let low_corner = *high_corner - Vector3::new(0.5, 2.0, 1.0);
    // TODO: this really shouldn't be Stone.
    Block::new(&low_corner, high_corner, Stone)
  }

  // move the player by a vector
  pub fn translate(&mut self, v: &Vector3<GLfloat>) {
    let player = self.construct_player(&(self.camera_position + *v));
    let mut collided = false;
    let mut i = 0;
    while i < self.world_data.len() {
      match intersect(&player, self.world_data.get(i)) {
        Intersect(stop) => {
          collided = true;
          let d = *v * stop - *v;
          self.camera_speed = self.camera_speed + d;
          break;
        },
        NoIntersect => {}
      }
      i += 1;
    }

    if !collided {
      self.camera_position = self.camera_position + *v;
      self.translation_matrix = self.translation_matrix * translate(&-v);
      self.update_projection();
    }
  }

  #[inline]
  // rotate the player's view.
  pub fn rotate(&mut self, v: &Vector3<GLfloat>, r: angle::Rad<GLfloat>) {
    self.rotation_matrix = self.rotation_matrix * from_axis_angle(v, -r);
    self.update_projection();
  }

  pub fn drop(&self) {
    unsafe {
      gl::DeleteBuffers(1, &self.vbo);
      gl::DeleteVertexArrays(1, &self.vao);
    }
  }

  // axes

  // Return the "right" axis (i.e. the x-axis rotated to match you).
  pub fn right(&self) -> Vector3<GLfloat> {
    return Matrix3::from_axis_angle(&Vector3::unit_y(), self.lateral_rotation).mul_v(&Vector3::unit_x());
  }

  // Return the "forward" axis (i.e. the z-axis rotated to match you).
  pub fn forward(&self) -> Vector3<GLfloat> {
    return Matrix3::from_axis_angle(&Vector3::unit_y(), self.lateral_rotation).mul_v(&-Vector3::unit_z());
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
            fail!("{}", str::from_utf8(buf.slice(0, buf.len())).expect("ShaderInfoLog not valid utf8"));
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
            fail!("{}", str::from_utf8(buf.slice(0, buf.len())).expect("ProgramInfoLog not valid utf8"));
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
