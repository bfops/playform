pub use color::Color4;
use bounding_box::BoundingBox;
use cgmath::aabb::Aabb2;
use cgmath::angle;
use cgmath::array::Array2;
use cgmath::matrix::{Matrix, Matrix3, Matrix4};
use cgmath::num::{BaseFloat};
use cgmath::point::{Point2, Point3};
use cgmath::vector::{Vector, Vector3};
use cgmath::projection;
use cstr_cache::CStringCache;
use fontloader;
use libc::types::common::c95;
use piston;
use piston::*;
use gl;
use gl::types::*;
use sdl2_game_window::GameWindowSDL2;
use sdl2::mouse;
use stopwatch;
use std::collections::HashMap;
use std::mem;
use std::iter::range_inclusive;
use std::ptr;
use std::str;
use std::num;
use std::raw;
use vertex::{ColoredVertex,TextureVertex};

// TODO(cgaebel): How the hell do I get this to be exported from `mod stopwatch`?
macro_rules! time(
  ($timers:expr, $name:expr, $f:expr) => (
    unsafe { ($timers as *const stopwatch::TimerSet).to_option() }.unwrap().time($name, $f)
  );
)

static WINDOW_WIDTH: u32 = 800;
static WINDOW_HEIGHT: u32 = 600;

static TRIANGLES_PER_BLOCK: uint = 12;
static LINES_PER_BLOCK: uint = 12;
static VERTICES_PER_TRIANGLE: uint = 3;
static VERTICES_PER_LINE: uint = 2;
static TRIANGLE_VERTICES_PER_BLOCK: uint = TRIANGLES_PER_BLOCK * VERTICES_PER_TRIANGLE;
static LINE_VERTICES_PER_BLOCK: uint = LINES_PER_BLOCK * VERTICES_PER_LINE;

static MAX_WORLD_SIZE: uint = 20000;

static MAX_JUMP_FUEL: uint = 4;

/// A data structure which specifies how to pass data from opengl to the vertex
/// shaders.
pub struct VertexAttribData<'a> {
  /// Cooresponds to the shader's `input variable`.
  pub name: &'a str,
  /// The size (in floats) of this attribute.
  pub size: uint,
}

/// A fixed-capacity array of GLfloat-based structures passed to OpenGL.
pub struct GLBuffer<T> {
  vertex_array: u32,
  vertex_buffer: u32,
  length: uint,
  capacity: uint,
}

impl<T: Clone> GLBuffer<T> {
  #[inline]
  /// An empty `GLBuffer`.
  pub unsafe fn null() -> GLBuffer<T> {
    GLBuffer {
      vertex_array: -1 as u32,
      vertex_buffer: -1 as u32,
      length: 0,
      capacity: 0,
    }
  }

  #[inline]
  /// Creates a new array of objects on the GPU.
  pub unsafe fn new(shader_program: GLuint, attribs: &[VertexAttribData], capacity: uint) -> GLBuffer<T> {
    let mut vertex_array = 0;
    let mut vertex_buffer = 0;
    gl::GenVertexArrays(1, &mut vertex_array);
    gl::GenBuffers(1, &mut vertex_buffer);

    gl::BindVertexArray(vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);

    let mut offset = 0;
    for attrib in attribs.iter() {
      let shader_attrib = glGetAttribLocation(shader_program, attrib.name) as GLuint;
      if shader_attrib == -1 {
        fail!("shader attribute \"{}\" not found", attrib.name);
      }

      gl::EnableVertexAttribArray(shader_attrib);
      gl::VertexAttribPointer(
          shader_attrib,
          attrib.size as i32,
          gl::FLOAT,
          gl::FALSE as GLboolean,
          mem::size_of::<T>() as i32,
          ptr::null().offset(offset),
      );
      offset += (attrib.size * mem::size_of::<GLfloat>()) as int;
    }

    if offset != mem::size_of::<T>() as int {
      fail!("attribs are incorrectly sized!");
    }

    gl::BufferData(
      gl::ARRAY_BUFFER,
      (capacity * mem::size_of::<T>()) as GLsizeiptr,
      ptr::null(),
      gl::DYNAMIC_DRAW,
    );

    GLBuffer {
      vertex_array: vertex_array,
      vertex_buffer: vertex_buffer,
      length: 0,
      capacity: capacity,
    }
  }

  /// Analog of vec::Vector::swap_remove`, but for GLBuffer data.
  pub unsafe fn swap_remove(&mut self, span: uint, i: uint) {
    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    self.length -= span;
    let size = mem::size_of::<T>();
    let copy_size = (size * span) as uint;
    let mut bytes: Vec<u8> = Vec::with_capacity(copy_size);
    bytes.set_len(copy_size);
    gl::GetBufferSubData(
      gl::ARRAY_BUFFER,
      (self.length * size) as i64,
      copy_size as i64,
      mem::transmute(&bytes.as_mut_slice()[0]),
    );
    gl::BufferSubData(
      gl::ARRAY_BUFFER,
      (i * span * size) as i64,
      copy_size as i64,
      mem::transmute(&bytes.slice(0, bytes.len())[0]),
    );
  }

  #[inline]
  /// Add a set of triangles to the set of triangles to render.
  pub unsafe fn push(&mut self, vs: &[T]) {
    if self.length >= self.capacity {
      fail!("Overfilled GLBuffer: {} out of {}", self.length, self.capacity);
    }

    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    let vs_as_slice : raw::Slice<T> = mem::transmute(vs);

    assert_eq!(vs_as_slice.data as uint & 0x3, 0);

    let size = mem::size_of::<T>() as i64;
    gl::BufferSubData(
      gl::ARRAY_BUFFER,
      size * self.length as i64,
      size * vs_as_slice.len as i64,
      vs_as_slice.data as *const c95::c_void
    );

    self.length += vs.len();
  }

  #[inline]
  /// Draws all the queued triangles to the screen.
  pub fn draw(&self, mode: GLenum) {
    self.draw_slice(mode, 0, self.length);
  }

  /// Draw some subset of the triangle array.
  pub fn draw_slice(&self, mode: GLenum, start: uint, len: uint) {
    gl::BindVertexArray(self.vertex_array);
    gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);

    gl::DrawArrays(mode, start as i32, len as i32);
  }
}

#[unsafe_destructor]
impl<T> Drop for GLBuffer<T> {
  #[inline]
  fn drop(&mut self) {
    unsafe {
      gl::DeleteBuffers(1, &self.vertex_buffer);
      gl::DeleteVertexArrays(1, &self.vertex_array);
    }
  }
}

#[deriving(Clone)]
#[allow(missing_doc)]
pub enum BlockType {
  Grass,
  Dirt,
  Stone,
}

impl BlockType {
  fn to_color(&self) -> Color4<GLfloat> {
    match *self {
      Grass => Color4::of_rgba(0.0, 0.5,  0.0, 1.0),
      Dirt  => Color4::of_rgba(0.2, 0.15, 0.1, 1.0),
      Stone => Color4::of_rgba(0.5, 0.5,  0.5, 1.0),
    }
  }
}

#[deriving(Clone)]
/// A voxel-ish block in the game world.
pub struct Block {
  bounds: BoundingBox,
  // bounds of the Block
  block_type: BlockType,
  id: u32,
}

type Intersect = Vector3<GLfloat>;

enum Intersect1 {
  Within,
  Partial,
}

// Find whether two Blocks intersect.
fn intersect(b1: &BoundingBox, b2: &BoundingBox) -> Option<Intersect> {
  fn intersect1(x1l: GLfloat, x1h: GLfloat, x2l: GLfloat, x2h: GLfloat) -> Option<Intersect1> {
    if x1l > x2l && x1h <= x2h {
      Some(Within)
    } else if x1h > x2l && x2h > x1l {
      Some(Partial)
    } else {
      None
    }
  }

  let mut ret = true;
  let mut v = Vector3::ident();
  match intersect1(b1.low_corner.x, b1.high_corner.x, b2.low_corner.x, b2.high_corner.x) {
    Some(Within) => { },
    Some(Partial) => { v.x = 0.0; },
    None => { ret = false; },
  }
  match intersect1(b1.low_corner.y, b1.high_corner.y, b2.low_corner.y, b2.high_corner.y) {
    Some(Within) => { },
    Some(Partial) => { v.y = 0.0; },
    None => { ret = false; },
  }
  match intersect1(b1.low_corner.z, b1.high_corner.z, b2.low_corner.z, b2.high_corner.z) {
    Some(Within) => { },
    Some(Partial) => { v.z = 0.0; },
    None => { ret = false; },
  }

  if ret {
    Some(v)
  } else {
    None
  }
}

impl Block {
  fn new(low_corner: Vector3<GLfloat>, high_corner: Vector3<GLfloat>, block_type: BlockType, id: u32) -> Block {
    Block {
      bounds: BoundingBox { low_corner: low_corner, high_corner: high_corner },
      block_type: block_type,
      id: id,
    }
  }

  // Construct the faces of the block as triangles for rendering,
  // with a different color on each face (front, left, top, back, right, bottom).
  // Triangle vertices are in CCW order when viewed from the outside of
  // the cube, for rendering purposes.
  fn to_triangles(&self, c: [Color4<GLfloat>, ..6]) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BLOCK] {
    let (x1, y1, z1) = (self.bounds.low_corner.x, self.bounds.low_corner.y, self.bounds.low_corner.z);
    let (x2, y2, z2) = (self.bounds.high_corner.x, self.bounds.high_corner.y, self.bounds.high_corner.z);

    let vtx = |x: GLfloat, y: GLfloat, z: GLfloat, c: Color4<GLfloat>| -> ColoredVertex {
      ColoredVertex {
        position: Point3 { x: x, y: y, z: z },
        color: c
      }
    };

    [
      // front
      vtx(x1, y1, z1, c[0]), vtx(x1, y2, z1, c[0]), vtx(x2, y2, z1, c[0]),
      vtx(x1, y1, z1, c[0]), vtx(x2, y2, z1, c[0]), vtx(x2, y1, z1, c[0]),
      // left
      vtx(x1, y1, z2, c[1]), vtx(x1, y2, z2, c[1]), vtx(x1, y2, z1, c[1]),
      vtx(x1, y1, z2, c[1]), vtx(x1, y2, z1, c[1]), vtx(x1, y1, z1, c[1]),
      // top
      vtx(x1, y2, z1, c[2]), vtx(x1, y2, z2, c[2]), vtx(x2, y2, z2, c[2]),
      vtx(x1, y2, z1, c[2]), vtx(x2, y2, z2, c[2]), vtx(x2, y2, z1, c[2]),
      // back
      vtx(x2, y1, z2, c[3]), vtx(x2, y2, z2, c[3]), vtx(x1, y2, z2, c[3]),
      vtx(x2, y1, z2, c[3]), vtx(x1, y2, z2, c[3]), vtx(x1, y1, z2, c[3]),
      // right
      vtx(x2, y1, z1, c[4]), vtx(x2, y2, z1, c[4]), vtx(x2, y2, z2, c[4]),
      vtx(x2, y1, z1, c[4]), vtx(x2, y2, z2, c[4]), vtx(x2, y1, z2, c[4]),
      // bottom
      vtx(x1, y1, z2, c[5]), vtx(x1, y1, z1, c[5]), vtx(x2, y1, z1, c[5]),
      vtx(x1, y1, z2, c[5]), vtx(x2, y1, z1, c[5]), vtx(x2, y1, z2, c[5]),
    ]
  }

  #[inline]
  fn to_colored_triangles(&self) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BLOCK] {
    let colors = [self.block_type.to_color(), ..6];
    self.to_triangles(colors)
  }

  // Construct outlines for this Block, to sharpen the edges.
  fn to_outlines(&self) -> [ColoredVertex, ..VERTICES_PER_LINE * LINES_PER_BLOCK] {
    // distance from the block to construct the bounding outlines.
    let d = 0.002;
    let (x1, y1, z1) = (self.bounds.low_corner.x - d, self.bounds.low_corner.y - d, self.bounds.low_corner.z - d);
    let (x2, y2, z2) = (self.bounds.high_corner.x + d, self.bounds.high_corner.y + d, self.bounds.high_corner.z + d);
    let c = Color4::of_rgba(0.0, 0.0, 0.0, 1.0);

    let vtx = |x: GLfloat, y: GLfloat, z: GLfloat| -> ColoredVertex {
      ColoredVertex {
        position: Point3 { x: x, y: y, z: z },
        color: c
      }
    };

    [
      vtx(x1, y1, z1), vtx(x2, y1, z1),
      vtx(x1, y2, z1), vtx(x2, y2, z1),
      vtx(x1, y1, z2), vtx(x2, y1, z2),
      vtx(x1, y2, z2), vtx(x2, y2, z2),

      vtx(x1, y1, z1), vtx(x1, y2, z1),
      vtx(x2, y1, z1), vtx(x2, y2, z1),
      vtx(x1, y1, z2), vtx(x1, y2, z2),
      vtx(x2, y1, z2), vtx(x2, y2, z2),

      vtx(x1, y1, z1), vtx(x1, y1, z2),
      vtx(x2, y1, z1), vtx(x2, y1, z2),
      vtx(x1, y2, z1), vtx(x1, y2, z2),
      vtx(x2, y2, z1), vtx(x2, y2, z2),
    ]
  }
}

pub struct Player {
  bounds: BoundingBox,
  // speed; units are world coordinates
  speed: Vector3<GLfloat>,
  // acceleration; x/z units are relative to player facing
  accel: Vector3<GLfloat>,
  // this is depleted as we jump and replenished as we stand.
  jump_fuel: uint,
  // are we currently trying to jump? (e.g. holding the key).
  is_jumping: bool,
}

/// The whole application. Wrapped up in a nice frameworky struct for piston.
pub struct App {
  world_data: Vec<Block>,
  player: Player,
  // next block id to assign
  next_block_id: u32,
  // mapping of block_id to the block's index in OpenGL buffers
  block_id_to_index: HashMap<u32, uint>,
  // OpenGL buffers
  world_triangles: GLBuffer<ColoredVertex>,
  outlines: GLBuffer<ColoredVertex>,
  hud_triangles: GLBuffer<ColoredVertex>,
  texture_triangles: GLBuffer<TextureVertex>,
  textures: Vec<GLuint>,
  // OpenGL-friendly equivalent of world_data for selection/picking.
  selection_triangles: GLBuffer<ColoredVertex>,
  // OpenGL projection matrix components
  hud_matrix: Matrix4<GLfloat>,
  fov_matrix: Matrix4<GLfloat>,
  translation_matrix: Matrix4<GLfloat>,
  rotation_matrix: Matrix4<GLfloat>,
  lateral_rotation: angle::Rad<GLfloat>,
  // OpenGL shader "program" id.
  shader_program: u32,
  texture_shader: u32,

  // which mouse buttons are currently pressed
  mouse_buttons_pressed: Vec<piston::mouse::Button>,

  font: fontloader::FontLoader,
  scache: CStringCache,

  timers: stopwatch::TimerSet,
}

/// Create a 3D translation matrix.
pub fn translate(t: Vector3<GLfloat>) -> Matrix4<GLfloat> {
  Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    t.x, t.y, t.z, 1.0,
  )
}

/// Create a 3D perspective initialization matrix.
pub fn perspective(fovy: GLfloat, aspect: GLfloat, near: GLfloat, far: GLfloat) -> Matrix4<GLfloat> {
  Matrix4::new(
    fovy / aspect, 0.0, 0.0,                              0.0,
    0.0,          fovy, 0.0,                              0.0,
    0.0,           0.0, (near + far) / (near - far),     -1.0,
    0.0,           0.0, 2.0 * near * far / (near - far),  0.0,
  )
}

#[inline]
/// Create a XY symmetric ortho matrix.
pub fn sortho(dx: GLfloat, dy: GLfloat, near: GLfloat, far: GLfloat) -> Matrix4<GLfloat> {
  projection::ortho(-dx, dx, -dy, dy, near, far)
}

/// Create a matrix from a rotation around an arbitrary axis.
pub fn from_axis_angle<S: BaseFloat>(axis: Vector3<S>, angle: angle::Rad<S>) -> Matrix4<S> {
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

/// Gets the id number for a given input of the shader program.
#[allow(non_snake_case_functions)]
pub unsafe fn glGetAttribLocation(shader_program: GLuint, name: &str) -> GLint {
  name.with_c_str(|ptr| gl::GetAttribLocation(shader_program, ptr))
}

#[inline]
pub fn swap_remove_first<T: PartialEq + Copy>(v: &mut Vec<T>, t: T) {
  match v.iter().position(|x| { *x == t }) {
    None => { },
    Some(i) => { v.swap_remove(i); },
  }
}

impl Game<GameWindowSDL2> for App {
  fn key_press(&mut self, _: &mut GameWindowSDL2, args: &KeyPressArgs) {
    time!(&self.timers, "event.key_press", || unsafe {
      match args.key {
        piston::keyboard::A => {
          self.walk(-Vector3::unit_x());
        },
        piston::keyboard::D => {
          self.walk(Vector3::unit_x());
        },
        piston::keyboard::LShift => {
          self.walk(-Vector3::unit_y());
        },
        piston::keyboard::Space => {
          if !self.player.is_jumping {
            self.player.is_jumping = true;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y + 0.3;
          }
        },
        piston::keyboard::W => {
          self.walk(-Vector3::unit_z());
        },
        piston::keyboard::S => {
          self.walk(Vector3::unit_z());
        },
        piston::keyboard::Left =>
          self.rotate_lateral(angle::rad(3.14 / 12.0 as GLfloat)),
        piston::keyboard::Right =>
          self.rotate_lateral(angle::rad(-3.14 / 12.0 as GLfloat)),
        piston::keyboard::Up =>
          self.rotate_vertical(angle::rad(3.14/12.0 as GLfloat)),
        piston::keyboard::Down =>
          self.rotate_vertical(angle::rad(-3.14/12.0 as GLfloat)),
        _ => {},
      }
    })
  }

  fn key_release(&mut self, _: &mut GameWindowSDL2, args: &KeyReleaseArgs) {
    time!(&self.timers, "event.key_release", || {
      match args.key {
        // accelerations are negated from those in key_press.
        piston::keyboard::A => {
          self.walk(Vector3::unit_x());
        },
        piston::keyboard::D => {
          self.walk(-Vector3::unit_x());
        },
        piston::keyboard::LShift => {
          self.walk(Vector3::unit_y());
        },
        piston::keyboard::Space => {
          if self.player.is_jumping {
            self.player.is_jumping = false;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y - 0.3;
          }
        },
        piston::keyboard::W => {
          self.walk(Vector3::unit_z());
        },
        piston::keyboard::S => {
          self.walk(-Vector3::unit_z());
        },
        _ => { }
      }
    })
  }

  #[inline]
  fn mouse_move(&mut self, w: &mut GameWindowSDL2, args: &MouseMoveArgs) {
    time!(&self.timers, "event.mouse_move", || unsafe {
      let (cx, cy) = (WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
      // args.y = h - args.y;
      // dy = args.y - cy;
      //  => dy = cy - args.y;
      let (dx, dy) = (args.x as f32 - cx, cy - args.y as f32);
      let (rx, ry) = (dx * -3.14 / 1024.0, dy * 3.14 / 1024.0);
      self.rotate_lateral(angle::rad(rx));
      self.rotate_vertical(angle::rad(ry));

      mouse::warp_mouse_in_window(&w.render_window.window, WINDOW_WIDTH as i32 / 2, WINDOW_HEIGHT as i32 / 2);
    })
  }

  #[inline]
  fn mouse_press(&mut self, _: &mut GameWindowSDL2, args: &MousePressArgs) {
    time!(&self.timers, "event.mouse_press", || {
      self.mouse_buttons_pressed.push(args.button);
    })
  }

  #[inline]
  fn mouse_release(&mut self, _: &mut GameWindowSDL2, args: &MouseReleaseArgs) {
    swap_remove_first(&mut self.mouse_buttons_pressed, args.button)
  }

  fn load(&mut self, _: &mut GameWindowSDL2) {
    time!(&self.timers, "load", || {
      mouse::show_cursor(false);

      gl::FrontFace(gl::CCW);
      gl::CullFace(gl::BACK);
      gl::Enable(gl::CULL_FACE);

      gl::Enable(gl::BLEND);
      gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

      gl::Enable(gl::LINE_SMOOTH);
      gl::LineWidth(2.5);

      gl::Enable(gl::DEPTH_TEST);
      gl::DepthFunc(gl::LESS);
      gl::ClearDepth(100.0);
      gl::ClearColor(0.0, 0.0, 0.0, 1.0);

      unsafe {
        self.set_up_shaders();

        // initialize the projection matrix
        self.fov_matrix = perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);
        self.translate(Vector3::new(0.0, 4.0, 10.0));
        self.update_projection();
      }

      let timers = &self.timers;

      unsafe {
        self.selection_triangles = GLBuffer::new(
          self.shader_program,
          [ VertexAttribData { name: "position", size: 3 },
            VertexAttribData { name: "in_color", size: 4 },
          ],
          MAX_WORLD_SIZE * TRIANGLE_VERTICES_PER_BLOCK,
        );

        self.world_triangles = GLBuffer::new(
          self.shader_program,
          [ VertexAttribData { name: "position", size: 3 },
            VertexAttribData { name: "in_color", size: 4 },
          ],
          MAX_WORLD_SIZE * TRIANGLE_VERTICES_PER_BLOCK,
        );

        self.outlines = GLBuffer::new(
          self.shader_program,
          [ VertexAttribData { name: "position", size: 3 },
            VertexAttribData { name: "in_color", size: 4 },
          ],
          MAX_WORLD_SIZE * LINE_VERTICES_PER_BLOCK,
        );

        self.hud_triangles = GLBuffer::new(
          self.shader_program,
          [ VertexAttribData { name: "position", size: 3 },
            VertexAttribData { name: "in_color", size: 4 },
          ],
          16 * VERTICES_PER_TRIANGLE,
        );

        self.texture_triangles = GLBuffer::new(
          self.texture_shader,
          [ VertexAttribData { name: "position", size: 2 },
            VertexAttribData { name: "texture_position", size: 2 },
          ],
          8 * VERTICES_PER_TRIANGLE,
        );


        self.make_textures();
        self.make_hud();
      }

      timers.time("load.construct", || unsafe {
        // low dirt block
        for i in range_inclusive(-1i, 1) {
          for j in range_inclusive(-1i, 1) {
            let (x1, y1, z1) = (3.0 + i as GLfloat, 6.0, 0.0 + j as GLfloat);
            let (x2, y2, z2) = (4.0 + i as GLfloat, 7.0, 1.0 + j as GLfloat);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Dirt, false);
          }
        }
        // high dirt block
        for i in range_inclusive(-1i, 1) {
          for j in range_inclusive(-1i, 1) {
            let (x1, y1, z1) = (0.0 + i as GLfloat, 12.0, 5.0 + j as GLfloat);
            let (x2, y2, z2) = (1.0 + i as GLfloat, 13.0, 6.0 + j as GLfloat);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Dirt, false);
          }
        }
        // ground
        for i in range_inclusive(-32i, 32) {
          for j in range_inclusive(-32i, 32) {
            let (x1, y1, z1) = (i as GLfloat - 0.5, 0.0, j as GLfloat - 0.5);
            let (x2, y2, z2) = (i as GLfloat + 0.5, 1.0, j as GLfloat + 0.5);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Grass, false);
          }
        }
        // front wall
        for i in range_inclusive(-32i, 32) {
          for j in range_inclusive(0i, 32) {
            let (x1, y1, z1) = (i as GLfloat - 0.5, 1.0 + j as GLfloat, -32.0 - 0.5);
            let (x2, y2, z2) = (i as GLfloat + 0.5, 2.0 + j as GLfloat, -32.0 + 0.5);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Stone, false);
          }
        }
        // back wall
        for i in range_inclusive(-32i, 32) {
          for j in range_inclusive(0i, 32) {
            let (x1, y1, z1) = (i as GLfloat - 0.5, 1.0 + j as GLfloat, 32.0 - 0.5);
            let (x2, y2, z2) = (i as GLfloat + 0.5, 2.0 + j as GLfloat, 32.0 + 0.5);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Stone, false);
          }
        }
        // left wall
        for i in range_inclusive(-32i, 32) {
          for j in range_inclusive(0i, 32) {
            let (x1, y1, z1) = (-32.0 - 0.5, 1.0 + j as GLfloat, i as GLfloat - 0.5);
            let (x2, y2, z2) = (-32.0 + 0.5, 2.0 + j as GLfloat, i as GLfloat + 0.5);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Stone, false);
          }
        }
        // right wall
        for i in range_inclusive(-32i, 32) {
          for j in range_inclusive(0i, 32) {
            let (x1, y1, z1) = (32.0 - 0.5, 1.0 + j as GLfloat, i as GLfloat - 0.5);
            let (x2, y2, z2) = (32.0 + 0.5, 2.0 + j as GLfloat, i as GLfloat + 0.5);
            self.place_block(Vector3::new(x1, y1, z1), Vector3::new(x2, y2, z2), Stone, false);
          }
        }
      });
    })
  }

  fn update(&mut self, _: &mut GameWindowSDL2, _: &UpdateArgs) {
    time!(&self.timers, "update", || unsafe {
      if self.player.is_jumping {
        if self.player.jump_fuel > 0 {
          self.player.jump_fuel -= 1;
        } else {
          // this code is duplicated in a few places
          self.player.is_jumping = false;
          self.player.accel.y = self.player.accel.y - 0.3;
        }
      }

      let dP = self.player.speed;
      if dP.x != 0.0 {
        self.translate(Vector3::new(dP.x, 0.0, 0.0));
      }
      if dP.y != 0.0 {
        self.translate(Vector3::new(0.0, dP.y, 0.0));
      }
      if dP.z != 0.0 {
        self.translate(Vector3::new(0.0, 0.0, dP.z));
      }

      let dV = Matrix3::from_axis_angle(&Vector3::unit_y(), self.lateral_rotation).mul_v(&self.player.accel);
      self.player.speed = self.player.speed + dV;
      // friction
      self.player.speed = self.player.speed * Vector3::new(0.7, 0.99, 0.7);

      // Block deletion
      if self.is_mouse_pressed(piston::mouse::Left) {
        time!(&self.timers, "update.delete_block", || unsafe {
          self
            .block_at_window_center()
            .map(|(block_index, _)| {
              self.remove_block(block_index);
            });
        })
      }
      if self.is_mouse_pressed(piston::mouse::Right) {
        unsafe {
          match self.block_at_window_center() {
            None => { },
            Some((block_index, face)) => {
              let block = self.world_data[block_index];
              let direction =
                    [ -Vector3::unit_z(),
                      -Vector3::unit_x(),
                       Vector3::unit_y(),
                       Vector3::unit_z(),
                       Vector3::unit_x(),
                      -Vector3::unit_y(),
                    ][face];
              self.place_block(
                block.bounds.low_corner + direction,
                block.bounds.high_corner + direction,
                Dirt,
                true
              );
            }
          }
        }
      }
    })
  }

  fn render(&mut self, _: &mut GameWindowSDL2, _: &RenderArgs) {
    time!(&self.timers, "render", || unsafe {
      // draw the world
      gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
      self.world_triangles.draw(gl::TRIANGLES);
      self.outlines.draw(gl::LINES);

      // draw the hud
      self.set_projection(&self.hud_matrix);
      self.hud_triangles.draw(gl::TRIANGLES);
      self.update_projection();

      // draw textures
      gl::UseProgram(self.texture_shader);
      let mut i = 0u;
      for tex in self.textures.iter() {
        gl::BindTexture(gl::TEXTURE_2D, *tex);
        self.texture_triangles.draw_slice(gl::TRIANGLES, i * 6, 6);
        i += 1;
      }
      gl::UseProgram(self.shader_program);
    })
  }
}

#[inline]
fn mask(mask: u32, i: u32) -> u32 {
  (i & mask) >> (mask as uint).trailing_zeros()
}

// map ids to unique colors
fn id_color(id: u32) -> Color4<GLfloat> {
  assert!(id < 0xFF000000, "too many items for selection buffer");
  let ret = Color4::of_rgba(
    (mask(0x00FF0000, id) as GLfloat / 255.0),
    (mask(0x0000FF00, id) as GLfloat / 255.0),
    (mask(0x000000FF, id) as GLfloat / 255.0),
    1.0,
  );
  assert!(ret.r >= 0.0);
  assert!(ret.r <= 1.0 as f32);
  assert!(ret.g >= 0.0 as f32);
  assert!(ret.g <= 1.0 as f32);
  assert!(ret.b >= 0.0 as f32);
  assert!(ret.b <= 1.0 as f32);
  ret
}

impl App {
  /// Initializes an empty app.
  pub unsafe fn new() -> App {
    App {
      world_data: Vec::new(),
      player: Player {
        bounds: BoundingBox {
            low_corner: Vector3::new(-1.0, -2.0, -1.0),
            high_corner: Vector3::zero(),
        },
        speed: Vector3::zero(),
        accel: Vector3::new(0.0, -0.1, 0.0),
        jump_fuel: 0,
        is_jumping: false,
      },
      // Start assigning block_ids at 1.
      // block_id 0 corresponds to no block.
      next_block_id: 1,
      block_id_to_index: HashMap::<u32, uint>::new(),
      world_triangles: GLBuffer::null(),
      outlines: GLBuffer::null(),
      hud_triangles: GLBuffer::null(),
      selection_triangles: GLBuffer::null(),
      texture_triangles: GLBuffer::null(),
      textures: Vec::new(),
      hud_matrix: translate(Vector3::new(0.0, 0.0, -1.0)) * sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0),
      fov_matrix: Matrix4::identity(),
      translation_matrix: Matrix4::identity(),
      rotation_matrix: Matrix4::identity(),
      lateral_rotation: angle::rad(0.0),
      shader_program: -1 as u32,
      texture_shader: -1 as u32,
      mouse_buttons_pressed: Vec::new(),
      font: fontloader::FontLoader::new(),
      scache: CStringCache::new(),
      timers: stopwatch::TimerSet::new(),
    }
  }

  /// Build all of our program's shaders.
  pub unsafe fn set_up_shaders(&mut self) {
    let ivs = compile_shader(ID_VS_SRC, gl::VERTEX_SHADER);
    let txs = compile_shader(TX_SRC, gl::FRAGMENT_SHADER);
    self.texture_shader = link_program(ivs, txs);
    gl::UseProgram(self.texture_shader);

    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    self.shader_program = link_program(vs, fs);
    gl::UseProgram(self.shader_program);
  }

  /// Makes some basic textures in the world.
  pub unsafe fn make_textures(&mut self) {
    let instructions = Vec::from_slice([
            "Use WASD to move, and spacebar to jump.",
            "Use the mouse to look around, and click to remove blocks."
        ]);

    let mut y = 0.99;

    for line in instructions.iter() {
      self.textures.push(self.font.sans.red(*line));

      self.texture_triangles.push(
        TextureVertex::square(
          Aabb2 {
            min: Point2 { x: -0.97, y: y - 0.2 },
            max: Point2 { x: 0.0,   y: y       },
          }));
      y -= 0.2;
    }
  }

  pub unsafe fn make_hud(&mut self) {
    let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

    self.hud_triangles.push(
      ColoredVertex::square(
        Aabb2 {
          min: Point2 { x: -0.02, y: -0.02 },
          max: Point2 { x:  0.02, y:  0.02 },
        }, cursor_color));
  }

  #[inline]
  pub fn is_mouse_pressed(&self, b: piston::mouse::Button) -> bool {
    self.mouse_buttons_pressed.iter().any(|x| { *x == b })
  }

  /// Sets the opengl projection matrix.
  pub unsafe fn set_projection(&mut self, m: &Matrix4<GLfloat>) {
    let var_name = self.scache.convert("proj_matrix").as_ptr();
    let loc = gl::GetUniformLocation(self.shader_program, var_name);
    assert!(loc != -1, "couldn't read matrix");
    gl::UniformMatrix4fv(loc, 1, 0, mem::transmute(m.ptr()));
  }

  #[inline]
  /// Updates the projetion matrix with all our movements.
  pub unsafe fn update_projection(&mut self) {
    time!(&self.timers, "update.projection", || {
      self.set_projection(&(self.fov_matrix * self.rotation_matrix * self.translation_matrix));
    })
  }

  #[inline]
  /// Renders the selection buffer.
  pub fn render_selection(&self) {
    time!(&self.timers, "render.render_selection", || {
      gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
      self.selection_triangles.draw(gl::TRIANGLES);
    })
  }

  /// Returns the index of the block at the given (x, y) coordinate in the window.
  /// The pixel coordinates are from (0, 0) to (WINDOW_WIDTH, WINDOW_HEIGHT).
  unsafe fn block_at_window(&self, x: i32, y: i32) -> Option<(uint, uint)> {
      self.render_selection();

      let pixels: Color4<u8> = Color4::of_rgba(0, 0, 0, 0);
      gl::ReadPixels(x, y, 1, 1, gl::RGB, gl::UNSIGNED_BYTE, mem::transmute(&pixels));

      let selection_id = (pixels.r as uint << 16) | (pixels.g as uint << 8) | (pixels.b as uint << 0);
      let block_id = selection_id / 6;
      let face_id = selection_id % 6;
      self.block_id_to_index.find(&(block_id as u32)).map(|&x| (x, face_id))
  }

  #[inline]
  /// Returns (block id, block face) shown at the center of the window.
  unsafe fn block_at_window_center(&self) -> Option<(uint, uint)> {
    self.block_at_window(WINDOW_WIDTH as i32 / 2, WINDOW_HEIGHT as i32 / 2)
  }

  #[inline]
  /// Find a collision with self.world_data.
  fn world_collision(&self, b: &BoundingBox) -> Option<Intersect> {
    for block in self.world_data.iter() {
      let i = intersect(b, &block.bounds);
      match i {
        None => { },
        Some(_) => { return i; },
      }
    }

    None
  }

  unsafe fn place_block(&mut self, low_corner: Vector3<GLfloat>, high_corner: Vector3<GLfloat>, block_type: BlockType, check_collisions: bool) {
    time!(&self.timers, "place_block", || {
      let block = Block::new(low_corner, high_corner, block_type, self.next_block_id);
      let collided = check_collisions &&
            ( self.world_collision(&block.bounds).is_some() || 
              intersect(&block.bounds, &self.player.bounds).is_some()
            );

      if !collided {
        self.world_data.grow(1, &block);
        self.world_triangles.push(block.to_colored_triangles());
        self.outlines.push(block.to_outlines());
        let selection_id = block.id * 6;
        let selection_colors =
              [ id_color(selection_id + 0),
                id_color(selection_id + 1),
                id_color(selection_id + 2),
                id_color(selection_id + 3),
                id_color(selection_id + 4),
                id_color(selection_id + 5),
              ];
        self.selection_triangles.push(block.to_triangles(selection_colors));
        self.block_id_to_index.insert(block.id, self.world_data.len() - 1);
        self.next_block_id += 1;
      }
    })
  }

  fn remove_block(&mut self, block_index: uint) {
    let block_id = self.world_data[block_index].id;
    // block that will be swapped into block_index in GL buffers after removal
    let swapped_block_id = self.world_data[self.world_data.len() - 1].id;
    unsafe {
      self.world_data.swap_remove(block_index);
      self.world_triangles.swap_remove(TRIANGLE_VERTICES_PER_BLOCK, block_index);
      self.outlines.swap_remove(LINE_VERTICES_PER_BLOCK, block_index);
      self.selection_triangles.swap_remove(TRIANGLE_VERTICES_PER_BLOCK, block_index);
    }
    self.block_id_to_index.remove(&block_id);
    if block_id != swapped_block_id {
      self.block_id_to_index.insert(swapped_block_id, block_index);
    }
  }

  #[inline]
  /// Changes the camera's acceleration by the given `da`.
  pub fn walk(&mut self, da: Vector3<GLfloat>) {
    self.player.accel = self.player.accel + da.mul_s(0.2);
  }

  /// Translates the camera by a vector.
  pub unsafe fn translate(&mut self, v: Vector3<GLfloat>) {
    let mut d_camera_speed : Vector3<GLfloat> = Vector3::new(0.0, 0.0, 0.0);

    let new_player_bounds = BoundingBox {
          low_corner: self.player.bounds.low_corner + v,
          high_corner: self.player.bounds.high_corner + v,
        };

    let collided =
      self
        .world_data
        .iter()
        .any(|block|
          match intersect(&new_player_bounds, &block.bounds) {
            Some(stop) => {
              d_camera_speed = v*stop - v;
              true
            }
            None => false,
          }
        );

    self.player.speed = self.player.speed + d_camera_speed;

    if collided {
      if v.y < 0.0 {
        self.player.jump_fuel = MAX_JUMP_FUEL;
      }
    } else {
      self.player.bounds = new_player_bounds;
      self.translation_matrix = self.translation_matrix * translate(-v);
      self.update_projection();

      if v.y < 0.0 {
        self.player.jump_fuel = 0;
      }
    }
  }

  #[inline]
  /// Rotate the player's view about a given vector, by `r` radians.
  pub unsafe fn rotate(&mut self, v: Vector3<GLfloat>, r: angle::Rad<GLfloat>) {
    self.rotation_matrix = self.rotation_matrix * from_axis_angle(v, -r);
    self.update_projection();
  }

  #[inline]
  /// Rotate the camera around the y axis, by `r` radians. Positive is
  /// counterclockwise.
  pub unsafe fn rotate_lateral(&mut self, r: angle::Rad<GLfloat>) {
    self.lateral_rotation = self.lateral_rotation + r;
    self.rotate(Vector3::unit_y(), r);
  }

  #[inline]
  /// Changes the camera pitch by `r` radians. Positive is up.
  pub unsafe fn rotate_vertical(&mut self, r: angle::Rad<GLfloat>) {
    let axis = self.right();
    self.rotate(axis, r);
  }

  // axes

  /// Return the "right" axis (i.e. the x-axis rotated to match you).
  pub fn right(&self) -> Vector3<GLfloat> {
    return Matrix3::from_axis_angle(&Vector3::unit_y(), self.lateral_rotation).mul_v(&Vector3::unit_x());
  }

  /// Return the "forward" axis (i.e. the z-axis rotated to match you).
  #[allow(dead_code)]
  pub fn forward(&self) -> Vector3<GLfloat> {
    return Matrix3::from_axis_angle(&Vector3::unit_y(), self.lateral_rotation).mul_v(&-Vector3::unit_z());
  }
}

// TODO(cgabeel): This should be removed when rustc bug #8861 is patched.
#[unsafe_destructor]
impl Drop for App {
  fn drop(&mut self) {
    if self.textures.len() == 0 { return }
    unsafe { gl::DeleteTextures(self.textures.len() as i32, &self.textures[0]); }
  }
}

// Shader sources
static VS_SRC: &'static str =
r"#version 330 core
uniform mat4 proj_matrix;

in  vec3 position;
in  vec4 in_color;
out vec4 color;

void main() {
  gl_Position = proj_matrix * vec4(position, 1.0);
  color = in_color;
}";

static FS_SRC: &'static str =
r"#version 330 core
in  vec4 color;
out vec4 frag_color;
void main() {
  frag_color = color;
}";

static ID_VS_SRC: &'static str =
r"#version 330 core
in  vec2 position;
in  vec2 texture_position;
out vec2 tex_position;
void main() {
  tex_position = texture_position;
  gl_Position = vec4(position, -1.0, 1.0);
}";

static TX_SRC: &'static str =
r"#version 330 core
in  vec2 tex_position;
out vec4 frag_color;

uniform sampler2D texture_in;

void main(){
  frag_color = texture(texture_in, vec2(tex_position.x, 1.0 - tex_position.y));
}
";

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

#[allow(dead_code)]
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

pub fn main() {
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

  let mut app = unsafe { App::new() };
  app.run(&mut window, &GameIteratorSettings {
    updates_per_second: 30,
    max_frames_per_second: 60,
  });

  println!("finished!");
  println!("");
  println!("runtime stats:");

  app.timers.print();
}
