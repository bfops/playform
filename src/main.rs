#![feature(globs)] // Allow global imports

extern crate cgmath;
extern crate gl;
extern crate piston;
extern crate sdl2_game_window;

use cgmath::angle;
use cgmath::array::Array2;
use cgmath::matrix::{Matrix, Matrix3, Matrix4};
use cgmath::num::{BaseFloat};
use cgmath::vector::{Vector, Vector2, Vector3};
use piston::*;
use gl::types::*;
use sdl2_game_window::GameWindowSDL2;
use std::mem;
use std::ptr;
use std::str;
use std::num;

mod stopwatch;

static WINDOW_WIDTH: u32 = 800;
static WINDOW_HEIGHT: u32 = 600;

static TRIANGLES_PER_BLOCK: uint = 12;
static LINES_PER_BLOCK: uint = 12;
static VERTICES_PER_TRIANGLE: uint = 3;
static VERTICES_PER_LINE: uint = 2;

#[deriving(Clone)]
pub struct Color4<T> { r: T, g: T, b: T, a: T }

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

#[deriving(Clone)]
// Rendering vertex: position and color.
pub struct Vertex {
  position: Vector3<GLfloat>,
  color: Color4<GLfloat>,
}

impl Vertex {
  fn new(x: &GLfloat, y: &GLfloat, z: &GLfloat, c: &Color4<GLfloat>) -> Vertex {
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
  fn to_triangles(&self, c: &Color4<GLfloat>) -> [Vertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BLOCK] {
    let (x1, y1, z1) = (self.low_corner.x, self.low_corner.y, self.low_corner.z);
    let (x2, y2, z2) = (self.high_corner.x, self.high_corner.y, self.high_corner.z);
    [
      // front
      Vertex::new(&x1, &y1, &z1, c), Vertex::new(&x1, &y2, &z1, c), Vertex::new(&x2, &y2, &z1, c),
      Vertex::new(&x1, &y1, &z1, c), Vertex::new(&x2, &y2, &z1, c), Vertex::new(&x2, &y1, &z1, c),
      // left
      Vertex::new(&x1, &y1, &z2, c), Vertex::new(&x1, &y2, &z2, c), Vertex::new(&x1, &y2, &z1, c),
      Vertex::new(&x1, &y1, &z2, c), Vertex::new(&x1, &y2, &z1, c), Vertex::new(&x1, &y1, &z1, c),
      // top
      Vertex::new(&x1, &y2, &z1, c), Vertex::new(&x1, &y2, &z2, c), Vertex::new(&x2, &y2, &z2, c),
      Vertex::new(&x1, &y2, &z1, c), Vertex::new(&x2, &y2, &z2, c), Vertex::new(&x2, &y2, &z1, c),
      // back
      Vertex::new(&x2, &y1, &z2, c), Vertex::new(&x2, &y2, &z2, c), Vertex::new(&x1, &y2, &z2, c),
      Vertex::new(&x2, &y1, &z2, c), Vertex::new(&x1, &y2, &z2, c), Vertex::new(&x1, &y1, &z2, c),
      // right
      Vertex::new(&x2, &y1, &z1, c), Vertex::new(&x2, &y2, &z1, c), Vertex::new(&x2, &y2, &z2, c),
      Vertex::new(&x2, &y1, &z1, c), Vertex::new(&x2, &y2, &z2, c), Vertex::new(&x2, &y1, &z2, c),
      // bottom
      Vertex::new(&x1, &y1, &z2, c), Vertex::new(&x1, &y1, &z1, c), Vertex::new(&x2, &y1, &z1, c),
      Vertex::new(&x1, &y1, &z2, c), Vertex::new(&x2, &y1, &z1, c), Vertex::new(&x2, &y1, &z2, c),
    ]
  }

  #[inline]
  fn to_colored_triangles(&self) -> [Vertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BLOCK] {
    self.to_triangles(&self.block_type.to_color())
  }

  // Construct outlines for this Block, to sharpen the edges.
  fn to_outlines(&self) -> [Vertex, ..VERTICES_PER_LINE * LINES_PER_BLOCK] {
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
  mouse_position: Vector2<f64>,
  // OpenGL render-ready equivalent of world_data
  triangles: Vec<Vertex>,
  outlines: Vec<Vertex>,
  // OpenGL-friendly equivalent of world_data for selection/picking.
  selection_triangles: Vec<Vertex>,
  // OpenGL projection matrix components
  fov_matrix: Matrix4<GLfloat>,
  translation_matrix: Matrix4<GLfloat>,
  rotation_matrix: Matrix4<GLfloat>,
  lateral_rotation: angle::Rad<GLfloat>,
  // OpenGL shader "program" id.
  shader_program: u32,
  // OpenGL Vertex Array Object id(s).
  render_vertex_array: u32,
  selection_vertex_array: u32,
  // OpenGL Vertex Buffer Object id(s).
  render_vertex_buffer: u32,
  selection_vertex_buffer: u32,

  load_stopwatch: stopwatch::Stopwatch,
  load_construct_stopwatch: stopwatch::Stopwatch,
  key_press_stopwatch: stopwatch::Stopwatch,
  key_release_stopwatch: stopwatch::Stopwatch,
  mouse_move_stopwatch: stopwatch::Stopwatch,
  mouse_press_stopwatch: stopwatch::Stopwatch,
  update_projection_stopwatch: stopwatch::Stopwatch,
  make_render_data_stopwatch: stopwatch::Stopwatch,
  update_render_data_stopwatch: stopwatch::Stopwatch,
  update_render_data_construct_stopwatch: stopwatch::Stopwatch,
  update_render_data_buffer_stopwatch: stopwatch::Stopwatch,
  render_selection_load_unload_stopwatch: stopwatch::Stopwatch,
  render_selection_render_stopwatch: stopwatch::Stopwatch,
  render_selection_stopwatch: stopwatch::Stopwatch,
  update_stopwatch: stopwatch::Stopwatch,
  render_stopwatch: stopwatch::Stopwatch,
}

// Create a 3D translation matrix.
pub fn translate(t: &Vector3<GLfloat>) -> Matrix4<GLfloat> {
  Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    t.x, t.y, t.z, 1.0,
  )
}

// Create a 3D perspective initialization matrix.
pub fn perspective(fovy: GLfloat, aspect: GLfloat, near: GLfloat, far: GLfloat) -> Matrix4<GLfloat> {
  Matrix4::new(
    fovy / aspect, 0.0, 0.0,                              0.0,
    0.0,          fovy, 0.0,                              0.0,
    0.0,           0.0, (near + far) / (near - far),     -1.0,
    0.0,           0.0, 2.0 * near * far / (near - far),  0.0,
  )
}

// Create a matrix from a rotation around an arbitrary axis
pub fn from_axis_angle<S: BaseFloat>(axis: &Vector3<S>, angle: angle::Rad<S>) -> Matrix4<S> {
    let (s, c) = angle::sin_cos(angle);
    let _1subc = num::one::<S>() - c;

    Matrix4::new(
        _1subc * axis.x * axis.x + c,
        _1subc * axis.x * axis.y + s * axis.z,
        _1subc * axis.x * axis.z - s * axis.y,
        num::zero(),

        _1subc * axis.x * axis.y - s * axis.z,
        _1subc * axis.y * axis.y + c,
        _1subc * axis.y * axis.z + s * axis.x,
        num::zero(),

        _1subc * axis.x * axis.z + s * axis.y,
        _1subc * axis.y * axis.z - s * axis.x,
        _1subc * axis.z * axis.z + c,
        num::zero(),

        num::zero(),
        num::zero(),
        num::zero(),
        num::one(),
    )
}

#[inline]
// treat a vector as though every `span` elements are a contiguous block,
// and swap_remove the `i`th block.
pub fn swap_remove_block<T>(v: &mut Vec<T>, span: uint, i: uint) {
  let dest = i * span;
  let mut i = dest + span;
  while i > dest {
    v.swap_remove(i - 1);
    i -= 1;
  }
}

pub unsafe fn glGetAttribLocation(shader_program: GLuint, name: &str) -> GLint {
  name.with_c_str(|ptr| gl::GetAttribLocation(shader_program, ptr))
}

impl Game for App {
  fn key_press(&mut self, args: &KeyPressArgs) {
    let mut watch = self.key_press_stopwatch;
    watch.timed(|| {
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
          self.camera_accel.y = self.camera_accel.y + 0.3;
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
    });
    self.key_press_stopwatch = watch;
  }

  fn key_release(&mut self, args: &KeyReleaseArgs) {
    let mut watch = self.key_press_stopwatch;
    watch.timed(|| {
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
          self.camera_accel.y = self.camera_accel.y - 0.3;
        },
        piston::keyboard::W => {
          self.walk(&Vector3::unit_z());
        },
        piston::keyboard::S => {
          self.walk(&-Vector3::unit_z());
        },
        _ => { }
      }
    });
    self.key_release_stopwatch = watch;
  }

  fn mouse_move(&mut self, args: &MouseMoveArgs) {
    let mut watch = self.mouse_move_stopwatch;
    watch.timed(|| {
      self.mouse_position = Vector2::new(args.x, args.y);
    });
    self.mouse_move_stopwatch = watch;
  }

  fn mouse_press(&mut self, _: &MousePressArgs) {
    let mut watch = self.mouse_press_stopwatch;
    watch.timed(|| {
      self.render_selection();

      let pixels: Color4<u8> = Color4::new(&0, &0, &0, &0);
      unsafe {
        gl::ReadPixels(
          self.mouse_position.x as i32,
          WINDOW_HEIGHT as i32 - self.mouse_position.y as i32,
          1,
          1,
          gl::RGB,
          gl::UNSIGNED_BYTE,
          mem::transmute(&pixels)
        );
      }

      let block_index = (pixels.r as uint << 16) | (pixels.g as uint << 8) | (pixels.b as uint << 0);
      if block_index > 0 {
        let block_index = block_index - 1;
        self.world_data.swap_remove(block_index);
        
        swap_remove_block(&mut self.triangles, TRIANGLES_PER_BLOCK * VERTICES_PER_TRIANGLE, block_index);
        swap_remove_block(&mut self.outlines, LINES_PER_BLOCK * VERTICES_PER_LINE, block_index);
        swap_remove_block(&mut self.selection_triangles, TRIANGLES_PER_BLOCK * VERTICES_PER_TRIANGLE, block_index);
        
        self.update_render_data();
      }
    });
    self.mouse_press_stopwatch = watch;
  }

  fn load(&mut self) {
    let mut watch = self.load_stopwatch;
    watch.timed(|| {
      unsafe {
        self.set_up_shaders();

        let pos_attr = glGetAttribLocation(self.shader_program, "position");
        let color_attr = glGetAttribLocation(self.shader_program, "in_color");

        // Create Vertex Array Objects(s).
        gl::GenVertexArrays(1, &mut self.render_vertex_array);
        gl::GenVertexArrays(1, &mut self.selection_vertex_array);

        // Create Vertex Buffer Object(s).
        gl::GenBuffers(1, &mut self.render_vertex_buffer);
        gl::GenBuffers(1, &mut self.selection_vertex_buffer);

        // Set up the selection VAO/VBO.

        gl::BindVertexArray(self.selection_vertex_array);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.selection_vertex_buffer);

        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::EnableVertexAttribArray(color_attr as GLuint);

        // selection position data
        gl::VertexAttribPointer(
            pos_attr as GLuint,
            (mem::size_of::<Vector3<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            mem::size_of::<Vertex>() as i32,
            ptr::null(),
        );
        // selection color data
        gl::VertexAttribPointer(
            color_attr as GLuint,
            (mem::size_of::<Color4<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            mem::size_of::<Vertex>() as i32,
            ptr::null().offset(mem::size_of::<Vector3<GLfloat>>() as int),
        );

        // Set up the rendering VAO/VBO.

        gl::BindVertexArray(self.render_vertex_array);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.render_vertex_buffer);

        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::EnableVertexAttribArray(color_attr as GLuint);

        // rendered position data
        gl::VertexAttribPointer(
            pos_attr as GLuint,
            (mem::size_of::<Vector3<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            mem::size_of::<Vertex>() as i32,
            ptr::null(),
        );
        // rendered color data
        gl::VertexAttribPointer(
            color_attr as GLuint,
            (mem::size_of::<Color4<GLfloat>>() / mem::size_of::<GLfloat>()) as i32,
            gl::FLOAT,
            gl::FALSE as GLboolean,
            mem::size_of::<Vertex>() as i32,
            ptr::null().offset(mem::size_of::<Vector3<GLfloat>>() as int),
        );
      }

      gl::Enable(gl::DEPTH_TEST);
      gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

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

      let mut watch = self.load_construct_stopwatch;
      watch.timed(|| {
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
      });
      self.load_construct_stopwatch = watch;

      self.make_render_data();
      self.update_render_data();
    });
    self.load_stopwatch = watch;
  }

  fn update(&mut self, _:&UpdateArgs) {
    let mut watch = self.update_stopwatch;
    watch.timed(|| {
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
      self.camera_speed = self.camera_speed * Vector3::new(0.8, 0.99, 0.8);
    });
    self.update_stopwatch = watch;
  }

  fn render(&mut self, _:&RenderArgs) {
    let mut watch = self.render_stopwatch;
    watch.timed(|| {
      gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

      gl::DrawArrays(gl::TRIANGLES, 0, self.triangles.len() as i32);
      gl::DrawArrays(gl::LINES, self.triangles.len() as GLint, self.outlines.len() as i32);
    });
    self.render_stopwatch = watch;
  }
}

#[inline]
fn mask(mask: u32, i: u32) -> u32 {
  #[inline]
  fn ctz(n: u32) -> uint {
    let mut n = n;
    let mut ret = 0;
    while (n & 1) == 0 {
      n >>= 1;
      ret += 1;
    }
    ret
  }

  (i & mask) >> ctz(mask)
}

impl App {
  pub fn new() -> App {
    App {
      world_data: Vec::new(),
      camera_position: Vector3::zero(),
      camera_speed: Vector3::zero(),
      camera_accel: Vector3::new(0.0, -0.1, 0.0),
      mouse_position: Vector2::new(0.0, 0.0),
      triangles: Vec::new(),
      outlines: Vec::new(),
      selection_triangles: Vec::new(),
      fov_matrix: Matrix4::identity(),
      translation_matrix: Matrix4::identity(),
      rotation_matrix: Matrix4::identity(),
      lateral_rotation: angle::rad(0.0),
      shader_program: -1 as u32,
      render_vertex_array: -1 as u32,
      selection_vertex_array: -1 as u32,
      render_vertex_buffer: -1 as u32,
      selection_vertex_buffer: -1 as u32,
      load_stopwatch: stopwatch::Stopwatch::new(),
      load_construct_stopwatch: stopwatch::Stopwatch::new(),
      key_press_stopwatch: stopwatch::Stopwatch::new(),
      key_release_stopwatch: stopwatch::Stopwatch::new(),
      mouse_move_stopwatch: stopwatch::Stopwatch::new(),
      mouse_press_stopwatch: stopwatch::Stopwatch::new(),
      update_projection_stopwatch: stopwatch::Stopwatch::new(),
      make_render_data_stopwatch: stopwatch::Stopwatch::new(),
      update_render_data_stopwatch: stopwatch::Stopwatch::new(),
      update_render_data_construct_stopwatch: stopwatch::Stopwatch::new(),
      update_render_data_buffer_stopwatch: stopwatch::Stopwatch::new(),
      render_selection_stopwatch: stopwatch::Stopwatch::new(),
      render_selection_load_unload_stopwatch: stopwatch::Stopwatch::new(),
      render_selection_render_stopwatch: stopwatch::Stopwatch::new(),
      update_stopwatch: stopwatch::Stopwatch::new(),
      render_stopwatch: stopwatch::Stopwatch::new(),
    }
  }

  pub unsafe fn set_up_shaders(&mut self) {
    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);

    self.shader_program = link_program(vs, fs);
    gl::UseProgram(self.shader_program);
    "out_color".with_c_str(|ptr| gl::BindFragDataLocation(self.shader_program, 0, ptr));
  }

  pub fn render_selection(&mut self) {
    let mut watch = self.render_selection_stopwatch;
    watch.timed(|| {
      let mut watch = self.render_selection_load_unload_stopwatch;
      watch.timed(|| {
        // load the selection vertex array/buffer.
        gl::BindVertexArray(self.selection_vertex_array);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.selection_vertex_buffer);
      });
      self.render_selection_load_unload_stopwatch = watch;

      let mut watch = self.render_selection_render_stopwatch;
      watch.timed(|| {
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        gl::DrawArrays(gl::TRIANGLES, 0, self.selection_triangles.len() as i32);
      });
      self.render_selection_render_stopwatch = watch;

      let mut watch = self.render_selection_load_unload_stopwatch;
      watch.timed(|| {
        // reset the bound vertex array/buffer.
        gl::BindVertexArray(self.render_vertex_array);
        gl::BindBuffer(gl::ARRAY_BUFFER, self.render_vertex_buffer);
      });
      self.render_selection_load_unload_stopwatch = watch;
    });
    self.render_selection_stopwatch = watch;
  }

  // Update the OpenGL vertex data with the world data triangles.
  pub fn make_render_data(&mut self) {
    fn selection_color(i: u32) -> Color4<GLfloat> {
      assert!(i < 0xFF000000, "too many items for selection buffer");
      let i = i + 1;
      let ret = Color4::new(
        &(mask(0x00FF0000, i) as GLfloat / 255.0),
        &(mask(0x0000FF00, i) as GLfloat / 255.0),
        &(mask(0x000000FF, i) as GLfloat / 255.0),
        &0.0,
      );
      assert!(ret.r >= 0.0);
      assert!(ret.r <= 1.0);
      assert!(ret.g >= 0.0);
      assert!(ret.g <= 1.0);
      assert!(ret.b >= 0.0);
      assert!(ret.b <= 1.0);
      ret
    }

    let mut watch = self.make_render_data_stopwatch;
    watch.timed(|| {
      self.selection_triangles = Vec::new();

      self.triangles = Vec::new();
      self.triangles.reserve(self.world_data.len() * VERTICES_PER_TRIANGLE * TRIANGLES_PER_BLOCK);

      self.outlines = Vec::new();
      self.outlines.reserve(self.world_data.len() * VERTICES_PER_LINE * LINES_PER_BLOCK);

      let mut i = 0;
      while i < self.world_data.len() {
        let block = self.world_data[i];
        self.triangles.push_all(block.to_colored_triangles());
        self.outlines.push_all(block.to_outlines());
        self.selection_triangles.push_all(block.to_triangles(&selection_color(i as u32)));
        i += 1;
      }
    });
    self.make_render_data_stopwatch = watch;
  }

  fn update_render_data(&mut self) {
    let mut watch = self.update_render_data_stopwatch;
    watch.timed(|| {
      let mut render_data = Vec::new();
      let mut watch = self.update_render_data_construct_stopwatch;
      watch.timed(|| {
        render_data.push_all(self.triangles.slice(0, self.triangles.len()));
        render_data.push_all(self.outlines.slice(0, self.outlines.len()));
      });
      self.update_render_data_construct_stopwatch = watch;

      let mut watch = self.update_render_data_buffer_stopwatch;
      watch.timed(|| {
        unsafe {
          gl::BindVertexArray(self.selection_vertex_array);
          gl::BindBuffer(gl::ARRAY_BUFFER, self.selection_vertex_buffer);

          gl::BufferData(
            gl::ARRAY_BUFFER,
            (self.selection_triangles.len() * mem::size_of::<Vertex>()) as GLsizeiptr,
            mem::transmute(&self.selection_triangles[0]),
            gl::STATIC_DRAW);

          gl::BindVertexArray(self.render_vertex_array);
          gl::BindBuffer(gl::ARRAY_BUFFER, self.render_vertex_buffer);

          gl::BufferData(
            gl::ARRAY_BUFFER,
            (render_data.len() * mem::size_of::<Vertex>()) as GLsizeiptr,
            mem::transmute(&render_data[0]),
            gl::STATIC_DRAW
          );
        }
      });
      self.update_render_data_buffer_stopwatch = watch;
    });
    self.update_render_data_stopwatch = watch;
  }

  pub fn update_projection(&mut self) {
    let mut watch = self.update_projection_stopwatch;
    watch.timed(|| {
      unsafe {
        let loc = gl::GetUniformLocation(self.shader_program, "proj_matrix".to_c_str().unwrap());
        if loc == -1 {
          println!("couldn't read matrix");
        } else {
          let projection = self.fov_matrix * self.rotation_matrix * self.translation_matrix;
          gl::UniformMatrix4fv(loc, 1, 0, mem::transmute(projection.ptr()));
        }
      }
    });
    self.update_projection_stopwatch = watch;
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
      match intersect(&player, &self.world_data[i]) {
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
      gl::DeleteBuffers(1, &self.render_vertex_buffer);
      gl::DeleteVertexArrays(1, &self.render_vertex_array);
      gl::DeleteBuffers(1, &self.selection_vertex_buffer);
      gl::DeleteVertexArrays(1, &self.selection_vertex_array);
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
out vec4 color;\n\
void main() {\n\
gl_Position = proj_matrix * vec4(position, 1.0);\n\
color = in_color;\n\
}";

static FS_SRC: &'static str =
   "#version 150\n\
in vec4 color;\n\
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

fn print_stopwatch(name: &str, watch: &stopwatch::Stopwatch) {
  if watch.number_of_windows == 0 {
    println!("{} never ran", name);
  } else {
    println!(
      "{} avg {}ms over {} samples", name,
      (watch.total_time / watch.number_of_windows / 1000000),
      watch.number_of_windows
    );
  }
}

unsafe fn println_c_str(str: *const u8) {
  let mut str = str;
  loop {
    let c = *str as char;
    if c == '\0' {
      println!("");
      return;
    }
    print!("{:c}", c);
    str = str.offset(1);
  }
}

fn main() {
  println!("starting");

  let mut window = GameWindowSDL2::new(
    GameWindowSettings {
      title: "playform".to_string(),
      size: [WINDOW_WIDTH, WINDOW_HEIGHT],
      fullscreen: false,
      exit_on_esc: false,
    }
  );

  let opengl_version = gl::GetString(gl::VERSION);
  let glsl_version = gl::GetString(gl::SHADING_LANGUAGE_VERSION);
  print!("OpenGL version: ");
  unsafe { println_c_str(opengl_version); }
  print!("GLSL version: ");
  unsafe { println_c_str(glsl_version); }
  println!("");

  let mut app = App::new();
  app.run(&mut window, &GameIteratorSettings {
    updates_per_second: 30,
    max_frames_per_second: 30,
  });

  println!("finished!");
  println!("");
  println!("runtime stats:");
  print_stopwatch("load_stopwatch", &app.load_stopwatch);
  print_stopwatch("load_construct_stopwatch", &app.load_construct_stopwatch);
  print_stopwatch("key_press_stopwatch", &app.key_press_stopwatch);
  print_stopwatch("key_release_stopwatch", &app.key_release_stopwatch);
  print_stopwatch("mouse_move_stopwatch", &app.mouse_move_stopwatch);
  print_stopwatch("mouse_press_stopwatch", &app.mouse_press_stopwatch);
  print_stopwatch("update_projection_stopwatch", &app.update_projection_stopwatch);
  print_stopwatch("make_render_data_stopwatch", &app.make_render_data_stopwatch);
  print_stopwatch("update_render_data_stopwatch", &app.update_render_data_stopwatch);
  print_stopwatch("update_render_data_construct_stopwatch", &app.update_render_data_construct_stopwatch);
  print_stopwatch("update_render_data_buffer_stopwatch", &app.update_render_data_buffer_stopwatch);
  print_stopwatch("render_selection_stopwatch", &app.render_selection_stopwatch);
  print_stopwatch("render_selection_load_unload_stopwatch", &app.render_selection_load_unload_stopwatch);
  print_stopwatch("render_selection_render_stopwatch", &app.render_selection_render_stopwatch);
  print_stopwatch("update_stopwatch", &app.update_stopwatch);
  print_stopwatch("render_stopwatch", &app.render_stopwatch);
}
