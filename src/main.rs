pub use color::Color4;
use fontloader;
use ncollide3df32::bounding_volume::aabb::AABB;
use nalgebra::na::{Vec2, Vec3, RMul};
use octree;
use piston;
use piston::*;
use gl;
use glw;
use glw::{Camera,GLfloat,Lines,Triangles,Shader,Texture,GLBuffer,GLContext,translation};
use sdl2_game_window::GameWindowSDL2;
use sdl2::mouse;
use stopwatch;
use std::collections::HashMap;
use std::iter::range_inclusive;
use std::f32::consts::PI;
use std::rc::Rc;
use vertex;
use vertex::{ColoredVertex,TextureVertex};

// TODO(cgaebel): How the hell do I get this to be exported from `mod stopwatch`?
macro_rules! time(
  ($timers:expr, $name:expr, $f:expr) => (
    unsafe { ($timers as *const stopwatch::TimerSet).to_option() }.unwrap().time($name, $f)
  );
)

/// `expect` an Option with a message assuming it is the result of an entity
/// id lookup.
macro_rules! expect_id(
  ($v:expr) => (
    match $v {
      None => fail!("expected entity id not found"),
      Some(v) => v,
    }
  );
)

static WINDOW_WIDTH:  uint = 800;
static WINDOW_HEIGHT: uint = 600;

// much bigger than 200000 starts segfaulting.
static MAX_WORLD_SIZE: uint = 100000;

static MAX_JUMP_FUEL: uint = 4;

// how many blocks to load during every update step
static LOAD_SPEED:uint = 1 << 12;
static SKY_COLOR: Color4<GLfloat>  = Color4 {r: 0.2, g: 0.5, b: 0.7, a: 1.0 };

static TRIANGLES_PER_BOX: uint = 12;
static LINES_PER_BOX: uint = 12;
static VERTICES_PER_TRIANGLE: uint = 3;
static VERTICES_PER_LINE: uint = 2;
static TRIANGLE_VERTICES_PER_BOX: uint = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;
static LINE_VERTICES_PER_BOX: uint = LINES_PER_BOX * VERTICES_PER_LINE;

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

#[deriving(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
pub struct Id(u32);

impl Add<Id, Id> for Id {
  fn add(&self, rhs: &Id) -> Id {
    let Id(i1) = *self;
    let Id(i2) = *rhs;
    Id(i1 + i2)
  }
}

impl Mul<u32, Id> for Id {
  fn mul(&self, rhs: &u32) -> Id {
    let Id(i) = *self;
    Id(i * *rhs)
  }
}

// Construct the faces of the box as triangles for rendering,
// with a different color on each face (front, left, top, back, right, bottom).
// Triangle vertices are in CCW order when viewed from the outside of
// the cube, for rendering purposes.
fn to_triangles(bounds: &AABB, c: [Color4<GLfloat>, ..6]) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BOX] {
  let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
  let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

  let vtx = |x: GLfloat, y: GLfloat, z: GLfloat, c: Color4<GLfloat>| -> ColoredVertex {
    ColoredVertex {
      position: Vec3::new(x, y, z),
      color: c
    }
  };

  // Remember: x increases to the right, y increases up, and z becomes more
  // negative as depth from the viewer increases.
  [
    // front
    vtx(x2, y1, z2, c[3]), vtx(x2, y2, z2, c[3]), vtx(x1, y2, z2, c[3]),
    vtx(x2, y1, z2, c[3]), vtx(x1, y2, z2, c[3]), vtx(x1, y1, z2, c[3]),
    // left
    vtx(x1, y1, z2, c[1]), vtx(x1, y2, z2, c[1]), vtx(x1, y2, z1, c[1]),
    vtx(x1, y1, z2, c[1]), vtx(x1, y2, z1, c[1]), vtx(x1, y1, z1, c[1]),
    // top
    vtx(x1, y2, z1, c[2]), vtx(x1, y2, z2, c[2]), vtx(x2, y2, z2, c[2]),
    vtx(x1, y2, z1, c[2]), vtx(x2, y2, z2, c[2]), vtx(x2, y2, z1, c[2]),
    // back
    vtx(x1, y1, z1, c[0]), vtx(x1, y2, z1, c[0]), vtx(x2, y2, z1, c[0]),
    vtx(x1, y1, z1, c[0]), vtx(x2, y2, z1, c[0]), vtx(x2, y1, z1, c[0]),
    // right
    vtx(x2, y1, z1, c[4]), vtx(x2, y2, z1, c[4]), vtx(x2, y2, z2, c[4]),
    vtx(x2, y1, z1, c[4]), vtx(x2, y2, z2, c[4]), vtx(x2, y1, z2, c[4]),
    // bottom
    vtx(x1, y1, z2, c[5]), vtx(x1, y1, z1, c[5]), vtx(x2, y1, z1, c[5]),
    vtx(x1, y1, z2, c[5]), vtx(x2, y1, z1, c[5]), vtx(x2, y1, z2, c[5]),
  ]
}

#[deriving(Clone)]
/// A voxel-ish block in the game world.
pub struct Block {
  // bounds of the Block
  block_type: BlockType,
  id: Id,
}

impl Block {
  #[inline]
  fn to_triangles(block: &Block, bounds: &AABB) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BOX] {
    let colors = [block.block_type.to_color(), ..6];
    to_triangles(bounds, colors)
  }

  // Construct outlines for this Block, to sharpen the edges.
  fn to_outlines(bounds: &AABB) -> [ColoredVertex, ..VERTICES_PER_LINE * LINES_PER_BOX] {
    // distance from the block to construct the bounding outlines.
    let d = 0.002;
    let (x1, y1, z1) = (bounds.mins().x - d, bounds.mins().y - d, bounds.mins().z - d);
    let (x2, y2, z2) = (bounds.maxs().x + d, bounds.maxs().y + d, bounds.maxs().z + d);
    let c = Color4::of_rgba(0.0, 0.0, 0.0, 1.0);

    let vtx = |x: GLfloat, y: GLfloat, z: GLfloat| -> ColoredVertex {
      ColoredVertex {
        position: Vec3::new(x, y, z),
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
  camera: Camera,
  // speed; units are world coordinates
  speed: Vec3<GLfloat>,
  // acceleration; x/z units are relative to player facing
  accel: Vec3<GLfloat>,
  // this is depleted as we jump and replenished as we stand.
  jump_fuel: uint,
  // are we currently trying to jump? (e.g. holding the key).
  is_jumping: bool,
  id: Id,
}

struct BlockBuffers {
  id_to_index: HashMap<Id, uint>,
  index_to_id: Vec<Id>,

  triangles: GLBuffer<ColoredVertex>,
  outlines: GLBuffer<ColoredVertex>,

  // OpenGL-friendly equivalent of block data for selection/picking.
  selection_triangles: GLBuffer<ColoredVertex>,
}

impl BlockBuffers {
  pub unsafe fn new(gl: &GLContext, shader_program: &Rc<Shader>) -> BlockBuffers {
    BlockBuffers {
      id_to_index: HashMap::new(),
      index_to_id: Vec::new(),

      selection_triangles: GLBuffer::new(
        gl,
        shader_program.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "in_color", size: 4 },
        ],
        TRIANGLE_VERTICES_PER_BOX,
        MAX_WORLD_SIZE,
        Triangles
      ),

      triangles: GLBuffer::new(
        gl,
        shader_program.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "in_color", size: 4 },
        ],
        TRIANGLE_VERTICES_PER_BOX,
        MAX_WORLD_SIZE,
        Triangles
      ),

      outlines: GLBuffer::new(
        gl,
        shader_program.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "in_color", size: 4 },
        ],
        LINE_VERTICES_PER_BOX,
        MAX_WORLD_SIZE,
        Lines
      ),
    }
  }

  pub fn push(
    &mut self,
    id: Id,
    triangles: &[ColoredVertex],
    outlines: &[ColoredVertex],
    selections: &[ColoredVertex]
  ) {
    self.id_to_index.insert(id, self.index_to_id.len());
    self.index_to_id.push(id);

    self.triangles.push(triangles);
    self.outlines.push(outlines);
    self.selection_triangles.push(selections);
  }

  pub fn flush(&mut self, gl: &GLContext) {
    self.triangles.flush(gl);
    self.outlines.flush(gl);
    self.selection_triangles.flush(gl);
  }

  pub fn swap_remove(&mut self, gl: &GLContext, id: Id) {
    let idx = *expect_id!(self.id_to_index.find(&id));
    let swapped_id = self.index_to_id[self.index_to_id.len() - 1];
    self.index_to_id.swap_remove(idx).expect("ran out of blocks");
    self.triangles.swap_remove(gl, idx);
    self.selection_triangles.swap_remove(gl, idx);
    self.id_to_index.remove(&id);

    self.outlines.swap_remove(gl, idx);

    if id != swapped_id {
      self.id_to_index.insert(swapped_id, idx);
    }
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
    self.outlines.draw(gl);
  }

  pub fn draw_selection(&self, gl: &GLContext) {
    self.selection_triangles.draw(gl);
  }
}

/// The whole application. Wrapped up in a nice frameworky struct for piston.
pub struct App {
  world_space: octree::Octree<Id>,
  physics: HashMap<Id, AABB>,
  locations: HashMap<Id, *mut octree::Octree<Id>>,
  blocks: HashMap<Id, Block>,
  player: Player,
  // id of the next block to load
  next_load_id: Id,
  // next block id to assign
  next_id: Id,
  // OpenGL buffers
  block_buffers: Option<BlockBuffers>,
  hud_triangles: Option<GLBuffer<ColoredVertex>>,
  texture_triangles: Option<GLBuffer<TextureVertex>>,
  textures: Vec<Texture>,
  hud_camera: Camera,
  lateral_rotation: f32, // in radians
  vertical_rotation: f32, // in radians
  // OpenGL shader "program" id.
  shader_program: Option<Rc<Shader>>,
  texture_shader: Option<Rc<Shader>>,

  // which mouse buttons are currently pressed
  mouse_buttons_pressed: Vec<piston::mouse::Button>,

  font: fontloader::FontLoader,
  timers: stopwatch::TimerSet,
  gl: GLContext,
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
    time!(&self.timers, "event.key_press", || {
      match args.key {
        piston::keyboard::A => {
          self.walk(Vec3::new(-1.0, 0.0, 0.0));
        },
        piston::keyboard::D => {
          self.walk(Vec3::new(1.0, 0.0, 0.0));
        },
        piston::keyboard::LShift => {
          self.walk(Vec3::new(0.0, -1.0, 0.0));
        },
        piston::keyboard::Space => {
          if !self.player.is_jumping {
            self.player.is_jumping = true;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y + 0.3;
          }
        },
        piston::keyboard::W => {
          self.walk(Vec3::new(0.0, 0.0, -1.0));
        },
        piston::keyboard::S => {
          self.walk(Vec3::new(0.0, 0.0, 1.0));
        },
        piston::keyboard::Left =>
          self.rotate_lateral(PI / 12.0),
        piston::keyboard::Right =>
          self.rotate_lateral(-PI / 12.0),
        piston::keyboard::Up =>
          self.rotate_vertical(PI / 12.0),
        piston::keyboard::Down =>
          self.rotate_vertical(-PI / 12.0),
        _ => {},
      }
    })
  }

  fn key_release(&mut self, _: &mut GameWindowSDL2, args: &KeyReleaseArgs) {
    time!(&self.timers, "event.key_release", || {
      match args.key {
        // accelerations are negated from those in key_press.
        piston::keyboard::A => {
          self.walk(Vec3::new(1.0, 0.0, 0.0));
        },
        piston::keyboard::D => {
          self.walk(Vec3::new(-1.0, 0.0, 0.0));
        },
        piston::keyboard::LShift => {
          self.walk(Vec3::new(0.0, 1.0, 0.0));
        },
        piston::keyboard::Space => {
          if self.player.is_jumping {
            self.player.is_jumping = false;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y - 0.3;
          }
        },
        piston::keyboard::W => {
          self.walk(Vec3::new(0.0, 0.0, 1.0));
        },
        piston::keyboard::S => {
          self.walk(Vec3::new(0.0, 0.0, -1.0));
        },
        _ => { }
      }
    })
  }

  fn mouse_move(&mut self, w: &mut GameWindowSDL2, args: &MouseMoveArgs) {
    time!(&self.timers, "event.mouse_move", || {
      let (cx, cy) = (WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
      // args.y = h - args.y;
      // dy = args.y - cy;
      //  => dy = cy - args.y;
      let (dx, dy) = (args.x as f32 - cx, cy - args.y as f32);
      let (rx, ry) = (dx * -3.14 / 2048.0, dy * 3.14 / 1600.0);
      self.rotate_lateral(rx);
      self.rotate_vertical(ry);

      mouse::warp_mouse_in_window(&w.render_window.window, WINDOW_WIDTH as i32 / 2, WINDOW_HEIGHT as i32 / 2);
    })
  }

  fn mouse_press(&mut self, _: &mut GameWindowSDL2, args: &MousePressArgs) {
    time!(&self.timers, "event.mouse_press", || {
      self.mouse_buttons_pressed.push(args.button);
    })
  }

  fn mouse_release(&mut self, _: &mut GameWindowSDL2, args: &MouseReleaseArgs) {
    swap_remove_first(&mut self.mouse_buttons_pressed, args.button)
  }

  fn load(&mut self, _: &mut GameWindowSDL2) {
    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in initialization", err),
    }

    time!(&self.timers, "load", || {
      mouse::show_cursor(false);

      let playerId = self.alloc_id();
      self.player.id = playerId;
      let bounds =
        AABB::new(
          Vec3::new(-1.0, -2.0, -1.0),
          Vec3::new(0.0, 0.0, 0.0)
        );
      let octree_location = self.world_space.insert(bounds, playerId);
      self.locations.insert(playerId, octree_location);
      self.physics.insert(playerId, bounds);

      let gl = &mut self.gl;

      gl.enable_culling();
      gl.enable_alpha_blending();
      gl.enable_smooth_lines();
      gl.enable_depth_buffer();

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }

      self.set_up_shaders(gl);

      // initialize the projection matrix
      self.player.camera.fov = glw::perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }
      self.translate_player(Vec3::new(0.0, 4.0, 10.0));

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x}", err),
      }

      unsafe {
        self.block_buffers = Some(BlockBuffers::new(&self.gl, self.shader_program.get_ref()));

        self.hud_triangles = Some(GLBuffer::new(
            &self.gl,
            self.shader_program.get_ref().clone(),
            [ vertex::AttribData { name: "position", size: 3 },
            vertex::AttribData { name: "in_color", size: 4 },
            ],
            VERTICES_PER_TRIANGLE,
            16,
            Triangles
        ));

        self.texture_triangles = Some(GLBuffer::new(
            &self.gl,
            self.texture_shader.get_ref().clone(),
            [ vertex::AttribData { name: "position", size: 2 },
            vertex::AttribData { name: "texture_position", size: 2 },
            ],
            VERTICES_PER_TRIANGLE,
            8,
            Triangles,
        ));
      }

      self.make_textures(gl);
      self.make_hud(gl);
      self.make_world();
    })

    println!("load() finished with {} blocks", self.blocks.len());
  }

  fn update(&mut self, _: &mut GameWindowSDL2, _: &UpdateArgs) {
    time!(&self.timers, "update", || {
      // TODO(cgabel): Ideally, the update thread should not be touching OpenGL.
      let gl = &mut self.gl;

      // if there are more blocks to be loaded, add them into the OpenGL buffers.
      if self.next_load_id < self.next_id {
        time!(&self.timers, "update.load", || {
          let mut i = 0;
          while i < LOAD_SPEED && self.next_load_id < self.next_id {
            self.blocks.find(&self.next_load_id).map(|block| {
              let bounds = expect_id!(self.physics.find(&self.next_load_id));
              self.block_buffers.get_mut_ref().push(
                block.id,
                Block::to_triangles(block, bounds),
                Block::to_outlines(bounds),
                to_selection_triangles(bounds, block.id)
              );
            });

            self.next_load_id = self.next_load_id + Id(1);
            i += 1;
          }

          self.block_buffers.get_mut_ref().flush(gl);
        });
      }

      time!(&self.timers, "update.player", || {
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
          self.translate_player(Vec3::new(dP.x, 0.0, 0.0));
        }
        if dP.y != 0.0 {
          self.translate_player(Vec3::new(0.0, dP.y, 0.0));
        }
        if dP.z != 0.0 {
          self.translate_player(Vec3::new(0.0, 0.0, dP.z));
        }

        let dV =
            glw::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation)
            .rmul(&self.player.accel);
        self.player.speed = self.player.speed + dV;
        // friction
        self.player.speed = self.player.speed * Vec3::new(0.7, 0.99, 0.7 as f32);
      });

      // Block deletion
      if self.is_mouse_pressed(piston::mouse::Left) {
        time!(&self.timers, "update.delete_block", || {
          self.block_at_window_center(gl).map(|(id, _)| { self.remove_block(gl, &id) });
        })
      }
      if self.is_mouse_pressed(piston::mouse::Right) {
        time!(&self.timers, "update.place_block", || {
          self.block_at_window_center(gl).map(|(block_id, face)| {
            let bounds = expect_id!(self.physics.find(&block_id));
            let direction =
                  [ Vec3::new(0.0, 0.0, -1.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(0.0, -1.0, 0.0),
                  ][face] * 0.5 as GLfloat;
            // TODO: think about how this should work when placing size A blocks
            // against size B blocks.
            self.place_block(
              bounds.mins() + direction,
              bounds.maxs() + direction,
              Dirt,
              true
            );
          });
        })
      }
    })
  }

  fn render(&mut self, _: &mut GameWindowSDL2, _: &RenderArgs) {
    time!(&self.timers, "render", || {
      let gl = &mut self.gl;

      gl.set_background_color(SKY_COLOR);
      gl.clear_buffer();

      // draw the world
      self.update_projection(gl);
      self.block_buffers.get_ref().draw(gl);

      // draw the hud
      self.shader_program.get_mut_ref().set_camera(gl, &self.hud_camera);
      self.hud_triangles.get_ref().draw(gl);

      // draw textures
      gl.use_shader(self.texture_shader.get_ref().deref(), |gl| {
        for (i, tex) in self.textures.iter().enumerate() {
          tex.bind_2d(gl);
          let verticies_in_a_square = 6;
          self.texture_triangles.get_ref().draw_slice(
            gl,
            i*verticies_in_a_square,
            verticies_in_a_square);
        }
      });
    })
  }
}

#[inline]
fn mask(mask: u32, i: u32) -> u32 {
  (i & mask) >> (mask as uint).trailing_zeros()
}

// map ids to unique colors
fn id_color(id: Id) -> Color4<GLfloat> {
  let Id(id) = id;
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

fn to_selection_triangles(bounds: &AABB, id: Id) -> [ColoredVertex, ..TRIANGLE_VERTICES_PER_BOX] {
  let selection_id = id * 6;
  let selection_colors =
        [ id_color(selection_id + Id(0)),
          id_color(selection_id + Id(1)),
          id_color(selection_id + Id(2)),
          id_color(selection_id + Id(3)),
          id_color(selection_id + Id(4)),
          id_color(selection_id + Id(5)),
        ];
  to_triangles(bounds, selection_colors)
}

impl App {
  /// Initializes an empty app.
  pub fn new(gl: GLContext) -> App {
    let world_bounds = AABB::new(
      Vec3 { x: -512.0, y: -32.0, z: -512.0 },
      Vec3 { x: 512.0, y: 512.0, z: 512.0 },
    );
    App {
      world_space: octree::Octree::new(&world_bounds),
      physics: HashMap::new(),
      locations: HashMap::new(),
      blocks: HashMap::new(),
      player: Player {
        camera: Camera::unit(),
        speed: Vec3::new(0.0, 0.0, 0.0),
        accel: Vec3::new(0.0, -0.1, 0.0),
        jump_fuel: 0,
        is_jumping: false,
        id: Id(0),
      },
      next_load_id: Id(1),
      // Start assigning block_ids at 1.
      // block_id 0 corresponds to no block.
      next_id: Id(1),
      block_buffers: None,
      hud_triangles: None,
      texture_triangles: None,
      textures: Vec::new(),
      hud_camera: {
        let mut c = Camera::unit();
        c.fov = glw::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
        c.fov = translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
        c
      },
      lateral_rotation: 0.0,
      vertical_rotation: 0.0,
      shader_program: None,
      texture_shader: None,
      mouse_buttons_pressed: Vec::new(),
      font: fontloader::FontLoader::new(),
      timers: stopwatch::TimerSet::new(),
      gl: gl,
    }
  }

  /// Build all of our program's shaders.
  fn set_up_shaders(&mut self, gl: &mut GLContext) {
    self.texture_shader = Some(Rc::new(Shader::new(gl, ID_VS_SRC, TX_SRC)));
    self.shader_program = Some(Rc::new(Shader::new(gl, VS_SRC, FS_SRC)));

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in set_up_shaders", err),
    }
  }

  /// Makes some basic textures in the world.
  fn make_textures(&mut self, gl: &GLContext) {
    let instructions = Vec::from_slice([
            "Use WASD to move, and spacebar to jump.",
            "Use the mouse to look around, and click to remove blocks."
        ]);

    let mut y = 0.99;

    for line in instructions.iter() {
      self.textures.push(self.font.sans.red(*line));

      self.texture_triangles.get_mut_ref().push(
        TextureVertex::square(
          Vec2 { x: -0.97, y: y - 0.2 },
          Vec2 { x: 0.0,   y: y       }
        ));
      y -= 0.2;
    }

    self.texture_triangles.get_mut_ref().flush(gl);
  }

  fn make_hud(&mut self, gl: &GLContext) {
    let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

    self.hud_triangles.get_mut_ref().push(
      ColoredVertex::square(
        Vec2 { x: -0.02, y: -0.02 },
        Vec2 { x:  0.02, y:  0.02 },
        cursor_color
      ));

    self.hud_triangles.get_mut_ref().flush(gl);
  }

  fn make_world(&mut self) {
    time!(&self.timers, "make_world", || {
      // low dirt block
      for i in range_inclusive(-2i, 2) {
        for j in range_inclusive(-2i, 2) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (6.0 + i, 6.0, 0.0 + j);
          let (x2, y2, z2) = (6.5 + i, 6.5, 0.5 + j);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Dirt, false);
        }
      }
      // high dirt block
      for i in range_inclusive(-2i, 2) {
        for j in range_inclusive(-2i, 2) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (0.0 + i, 12.0, 5.0 + j);
          let (x2, y2, z2) = (0.5 + i, 12.5, 5.5 + j);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Dirt, false);
        }
      }
      // ground
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(-64i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (i, 0.0, j);
          let (x2, y2, z2) = (i + 0.5, 0.5, j + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Grass, false);
        }
      }
      // front wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (i, 0.5 + j, -32.0);
          let (x2, y2, z2) = (i + 0.5, 1.0 + j, -32.0 + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Stone, false);
        }
      }
      // back wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (i, 0.5 + j, 32.0);
          let (x2, y2, z2) = (i + 0.5, 1.0 + j, 32.0 + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Stone, false);
        }
      }
      // left wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (-32.0, 0.5 + j, i);
          let (x2, y2, z2) = (-32.0 + 0.5, 1.0 + j, i + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Stone, false);
        }
      }
      // right wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (32.0, 0.5 + j, i);
          let (x2, y2, z2) = (32.0 + 0.5, 1.0 + j, i + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), Stone, false);
        }
      }
    });
  }

  #[inline]
  pub fn is_mouse_pressed(&self, b: piston::mouse::Button) -> bool {
    self.mouse_buttons_pressed.iter().any(|x| { *x == b })
  }

  /// Updates the projetion matrix with all our movements.
  pub fn update_projection(&self, gl: &mut GLContext) {
    time!(&self.timers, "update.projection", || {
      self.shader_program.get_ref().set_camera(gl, &self.player.camera);

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in update_projection", err),
      }
    });
  }

  #[inline]
  /// Renders the selection buffer.
  pub fn render_selection(&self, gl: &mut GLContext) {
    time!(&self.timers, "render.render_selection", || {
      gl.set_background_color(Color4::of_rgba(0.0, 0.0, 0.0, 0.0));
      gl.clear_buffer();

      self.update_projection(gl);
      self.block_buffers.get_ref().draw_selection(gl);
    })
  }

  fn find_octree_location(&self, id: Id) -> *mut octree::Octree<Id> {
    *(self
    .locations
    .find(&id)
    .expect("location prematurely deleted")
    )
  }

  /// Returns the id of the entity at the given (x, y) coordinate in the window.
  /// The pixel coordinates are from (0, 0) to (WINDOW_WIDTH, WINDOW_HEIGHT).
  fn block_at_window(&self, gl: &mut GLContext, x: uint, y: uint) -> Option<(Id, uint)> {
    self.render_selection(gl);

    let pixels: Color4<u8> = gl.read_pixels(x, y, WINDOW_HEIGHT as uint, WINDOW_WIDTH as uint);

    let selection_id = (pixels.r as u32 << 16) | (pixels.g as u32 << 8) | (pixels.b as u32 << 0);
    if selection_id == 0 {
      None
    } else {
      Some((Id(selection_id / 6), selection_id as uint % 6))
    }
  }

  /// Returns (block id, block face) shown at the center of the window.
  fn block_at_window_center(&self, gl: &mut GLContext) -> Option<(Id, uint)> {
    self.block_at_window(gl, WINDOW_WIDTH / 2, WINDOW_HEIGHT / 2)
  }

  /// Find a collision with a world object
  fn collides_with(&self, self_id: Id, b: &AABB) -> bool {
    time!(&self.timers, "world_collision", || {
      self.world_space.intersect(b, self_id)
    })
  }

  fn alloc_id(&mut self) -> Id {
    let id = self.next_id;
    self.next_id = self.next_id + Id(1);
    id
  }

  fn place_block(&mut self, min: Vec3<GLfloat>, max: Vec3<GLfloat>, block_type: BlockType, check_collisions: bool) {
    time!(&self.timers, "place_block", || {
      let mut block = Block {
        block_type: block_type,
        id: Id(0),
      };
      let bounds = AABB::new(min, max);
      let collided = check_collisions && self.collides_with(Id(0), &bounds);

      if !collided {
        block.id = self.alloc_id();
        let octree_loc =
          time!(&self.timers, "place_block.octree", || {
            self.world_space.insert(bounds.clone(), block.id)
          });
        self.locations.insert(block.id, octree_loc);
        self.physics.insert(block.id, bounds.clone());
        self.blocks.insert(block.id, block);
      }
    })
  }

  fn remove_block(&mut self, gl: &GLContext, id: &Id) {
    {
      let bounds = expect_id!(self.physics.find(id));
      let &octree_location = self.locations.find(id).expect("no octree_location to remove");
      self.locations.remove(id);
      unsafe {
        (*octree_location).remove(*id, bounds);
      }
    }
    self.blocks.remove(id);
    self.block_buffers.get_mut_ref().swap_remove(gl, *id);
    self.physics.remove(id);
  }

  /// Changes the camera's acceleration by the given `da`.
  fn walk(&mut self, da: Vec3<GLfloat>) {
    self.player.accel = self.player.accel + da * 0.2 as GLfloat;
  }

  /// Move an entity by some amount, returning the collisions that occur.
  /// If we don't collide, update self.locations with the moved object.
  /// Does NOT update any asociated GLbuffers, etc.
  fn translate(&mut self, id: Id, amount: Vec3<GLfloat>) -> bool {
    time!(&self.timers, "translate", || {
      let bounds = expect_id!(self.physics.find(&id));
      let new_bounds =
        AABB::new(
          bounds.mins() + amount,
          bounds.maxs() + amount
        );

      let octree_location = self.find_octree_location(id);
      assert!(octree_location.is_not_null());
      let collision = unsafe {
        (*octree_location).intersect(&new_bounds, id)
      };

      if !collision {
        unsafe {
          let octree_location = (*octree_location).move(id, bounds, new_bounds);
          self.locations.insert(id, octree_location);
          self.physics.insert(id, new_bounds);
        }
      }

      collision
    })
  }

  /// Translates the player/camera by a vector.
  fn translate_player(&mut self, v: Vec3<GLfloat>) {
    let id = self.player.id;
    let collisions = self.translate(id, v);
    if collisions {
      self.player.speed = self.player.speed - v;

      if v.y < 0.0 {
        self.player.jump_fuel = MAX_JUMP_FUEL;
      }
    } else {
      self.player.camera.translate(v);

      if v.y < 0.0 {
        self.player.jump_fuel = 0;
      }
    }
  }

  #[inline]
  /// Rotate the camera around the y axis, by `r` radians. Positive is
  /// counterclockwise.
  pub fn rotate_lateral(&mut self, r: GLfloat) {
    self.lateral_rotation = self.lateral_rotation + r;
    self.player.camera.rotate(Vec3::new(0.0, 1.0, 0.0), r);
  }

  /// Changes the camera pitch by `r` radians. Positive is up.
  /// Angles that "flip around" (i.e. looking too far up or down)
  /// are sliently rejected.
  pub fn rotate_vertical(&mut self, r: GLfloat) {
    let new_rotation = self.vertical_rotation + r;

    if new_rotation < -PI / 2.0
    || new_rotation >  PI / 2.0 {
      return
    }

    self.vertical_rotation = new_rotation;
    let axis = self.right();
    self.player.camera.rotate(axis, r);
  }

  // axes

  /// Return the "right" axis (i.e. the x-axis rotated to match you).
  pub fn right(&self) -> Vec3<GLfloat> {
    return
      glw::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation)
        .rmul(&Vec3::new(1.0, 0.0, 0.0))
  }

  /// Return the "forward" axis (i.e. the z-axis rotated to match you).
  #[allow(dead_code)]
  pub fn forward(&self) -> Vec3<GLfloat> {
    return
      glw::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation)
        .rmul(&Vec3::new(0.0, 0.0, -1.0))
  }
}

// TODO(cgaebel): This should be removed when rustc bug #8861 is patched.
#[unsafe_destructor]
impl Drop for App {
  fn drop(&mut self) {
    println!("Update Stats");
    println!("====================");
    self.timers.print();
    println!("");
  }
}

// Shader sources
static VS_SRC: &'static str =
r"#version 330 core
uniform mat4 projection_matrix;

in  vec3 position;
in  vec4 in_color;
out vec4 color;

void main() {
  gl_Position = projection_matrix * vec4(position, 1.0);
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


pub fn main() {
  println!("starting");

  let mut window = GameWindowSDL2::new(
    GameWindowSettings {
      title: "playform".to_string(),
      size: [WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32],
      fullscreen: false,
      exit_on_esc: false,
    }
  );

  let gl = GLContext::new();

  gl.print_stats();

  unsafe {
    App::new(gl).run(
      &mut window,
      &GameIteratorSettings {
        updates_per_second: 30,
        max_frames_per_second: 60,
      });
  }

  println!("finished!");
  println!("");
}
