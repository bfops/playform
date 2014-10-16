use borrow::*;
use common::*;
use event::{WindowSettings, Event, EventIterator, EventSettings, Update, Input, Render};
use fontloader;
use gl;
use gl::types::*;
use glw::camera;
use glw::camera::Camera;
use glw::color::Color4;
use glw::gl_buffer::*;
use glw::gl_context::GLContext;
use glw::light::Light;
use glw::queue::Queue;
use glw::shader::Shader;
use glw::texture::Texture;
use glw::vertex;
use glw::vertex::{ColoredVertex, TextureVertex};
use id_allocator::{Id, IdAllocator};
use input;
use input::{Press,Release,Move,Keyboard,Mouse,MouseCursor};
use libc::types::common::c95::c_void;
use loader::{Loader, Load, Unload};
use mob;
use nalgebra::{Pnt2, Vec2, Vec3, Pnt3, Norm};
use ncollide::math::Scalar;
use ncollide::bounding_volume::aabb::AABB;
use octree;
use physics::Physics;
use player::Player;
use png;
use sdl2_game_window::{WindowSDL2};
use sdl2::mouse;
use shader;
use shader_version::opengl::*;
use stopwatch;
use stopwatch::*;
use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::iter::range_inclusive;
use std::mem;
use std::raw;
use std::rc::Rc;
use terrain;

// how many terrain polys to load during every update step
static TERRAIN_LOAD_SPEED: uint = 1 << 11;
static OCTREE_LOAD_SPEED: uint = 1 << 11;
static SKY_COLOR: Color4<GLfloat>  = Color4 {r: 0.2, g: 0.5, b: 0.7, a: 1.0 };

static USE_LIGHTING: bool = false;

/// Defines volumes that can be tightened.
trait TightBoundingVolume {
  /// Reduce each of a volume's bounds by some amount.
  fn tightened(&self, amount: Scalar) -> Self;
}

impl TightBoundingVolume for AABB {
  fn tightened(&self, amount: Scalar) -> AABB {
    let mut new_min = self.mins() + Vec3::new(amount, amount, amount);
    let mut new_max = self.maxs() - Vec3::new(amount, amount, amount);
    if new_min.x > new_max.x {
      let mid = (new_min.x + new_max.x) / 2.0;
      new_min.x = mid;
      new_max.x = mid;
    }
    if new_min.y > new_max.x {
      let mid = (new_min.y + new_max.x) / 2.0;
      new_min.y = mid;
      new_max.y = mid;
    }
    if new_min.z > new_max.x {
      let mid = (new_min.z + new_max.x) / 2.0;
      new_min.z = mid;
      new_max.z = mid;
    }
    AABB::new(new_min, new_max)
  }
}

fn to_triangles(bounds: &AABB, c: &Color4<GLfloat>) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BOX] {
  let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
  let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

  let vtx = |x, y, z| {
    ColoredVertex {
      position: Pnt3::new(x, y, z),
      color: c.clone(),
    }
  };

  // Remember: x increases to the right, y increases up, and z becomes more
  // negative as depth from the viewer increases.
  [
    // front
    vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
    vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
    // left
    vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
    vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
    // top
    vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
    vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
    // back
    vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
    vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
    // right
    vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
    vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
    // bottom
    vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
    vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
  ]
}

macro_rules! translate_mob(
  ($world:expr, $mob:expr, $v:expr) => (
    App::translate_mob(
      &$world.gl,
      &mut $world.physics,
      &mut $world.mob_buffers,
      $mob,
      $v
    );
  );
)

fn center(bounds: &AABB) -> Pnt3<GLfloat> {
  (bounds.mins() + bounds.maxs().to_vec()) / (2.0 as GLfloat)
}

#[inline]
pub fn swap_remove_first<T: PartialEq + Copy>(v: &mut Vec<T>, t: T) {
  match v.iter().position(|x| *x == t) {
    None => { },
    Some(i) => { v.swap_remove(i); },
  }
}

fn load_terrain_textures() -> HashMap<terrain::TerrainType, Rc<Texture>> {
  let mut terrain_textures = HashMap::new();

  for &(terrain_type, path) in [
        (terrain::Grass, "textures/grass.png"),
        (terrain::Stone, "textures/stone.png"),
        (terrain::Dirt, "textures/dirt.png")
      ].iter() {
    let img = match png::load_png(&Path::new(path)) {
      Ok(i) => i,
      Err(s) => fail!("Could not load png {}: {}", path, s)
    };
    if img.color_type != png::RGBA8 {
      fail!("unsupported color type {:} in png", img.color_type);
    }
    debug!("loaded rgba8 png file {}", path);

    gl::ActiveTexture(gl::TEXTURE0 + terrain_type as GLuint);
    let texture = unsafe {
      let mut texture = 0;
      gl::GenTextures(1, &mut texture);
      gl::BindTexture(gl::TEXTURE_2D, texture);
      let pixels: raw::Slice<c_void> = mem::transmute(img.pixels.as_slice());
      gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, img.width as i32, img.height as i32,
                    0, gl::RGBA, gl::UNSIGNED_INT_8_8_8_8_REV, pixels.data);
      gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
      gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
      texture
    };

    let texture = Texture { id: texture };
    terrain_textures.insert(terrain_type, Rc::new(texture));
  }

  gl::ActiveTexture(gl::TEXTURE0 + terrain_textures.len() as GLuint);

  terrain_textures
}

fn make_text(
  gl: &GLContext,
  shader: Rc<RefCell<Shader>>,
) -> (Vec<Texture>, GLSliceBuffer<TextureVertex>) {
  let fontloader = fontloader::FontLoader::new();
  let mut textures = Vec::new();
  let mut triangles = {
    GLSliceBuffer::new(
        gl,
        shader,
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "texture_position", size: 2, unit: vertex::Float },
        ],
        VERTICES_PER_TRIANGLE,
        8,
        Triangles,
    )
  };

  let instructions = [
          "Use WASD to move, and spacebar to jump.",
          "Use the mouse to look around, and click to remove terrain."
      ].to_vec();

  let mut y = 0.99;

  for line in instructions.iter() {
    textures.push(fontloader.sans.red(*line));

    triangles.push(
      gl,
      TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }

  (textures, triangles)
}

fn make_hud(
  gl: &GLContext,
  shader: Rc<RefCell<Shader>>,
) -> GLSliceBuffer<ColoredVertex> {
  let mut hud_triangles = {
    GLSliceBuffer::new(
        gl,
        shader,
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
        ],
        VERTICES_PER_TRIANGLE,
        16,
        Triangles
    )
  };

  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  hud_triangles.push(
    gl,
    ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    )
  );

  hud_triangles
}

fn make_terrain(
  physics: &mut Physics<Id>,
  id_allocator: &mut IdAllocator,
) -> (HashMap<Id, terrain::TerrainPiece>, Loader<Id, Id>) {
  let mut terrains = HashMap::new();
  let mut terrain_loader = Queue::new(1 << 20);

  {
    let w = 1.0 / 2.0;
    let place_terrain = |bounds, vertices| {
      place_terrain(
        physics,
        &mut terrains,
        &mut terrain_loader,
        id_allocator,
        bounds,
        vertices,
        false
      );
    };

    enum Facing {
      Up,
      Down,
      Left,
      Right,
      Front,
      Back,
    }

    let place_square = |x: GLfloat, y: GLfloat, z: GLfloat, terrain_type: terrain::TerrainType, facing: Facing| {
      let vtx = |v| {
        terrain::TerrainVertex {
          position: v,
          terrain_type: terrain_type as GLuint,
        }
      };
      let (min, max, v1, v2, v3, v4) = match facing {
        Up => {
          let v1 = Pnt3::new(x, y, z);
          let v2 = Pnt3::new(x + w, y, z);
          let v3 = Pnt3::new(x + w, y, z + w);
          let v4 = Pnt3::new(x, y, z + w);
          (v1, v3, v2, v1, v4, v3)
        },
        Down => {
          let v1 = Pnt3::new(x, y, z);
          let v2 = Pnt3::new(x + w, y, z);
          let v3 = Pnt3::new(x + w, y, z + w);
          let v4 = Pnt3::new(x, y, z + w);
          (v1, v3, v1, v2, v3, v4)
        },
        Left => {
          let v1 = Pnt3::new(x, y, z);
          let v2 = Pnt3::new(x, y + w, z);
          let v3 = Pnt3::new(x, y + w, z + w);
          let v4 = Pnt3::new(x, y, z + w);
          (v1, v3, v1, v4, v3, v2)
        },
        Right => {
          let v1 = Pnt3::new(x, y, z);
          let v2 = Pnt3::new(x, y + w, z);
          let v3 = Pnt3::new(x, y + w, z + w);
          let v4 = Pnt3::new(x, y, z + w);
          (v1, v3, v4, v1, v2, v3)
        },
        Front => {
          let v1 = Pnt3::new(x, y, z);
          let v2 = Pnt3::new(x + w, y, z);
          let v3 = Pnt3::new(x + w, y + w, z);
          let v4 = Pnt3::new(x, y + w, z);
          (v1, v3, v2, v1, v4, v3)
        },
        Back => {
          let v1 = Pnt3::new(x, y, z);
          let v2 = Pnt3::new(x + w, y, z);
          let v3 = Pnt3::new(x + w, y + w, z);
          let v4 = Pnt3::new(x, y + w, z);
          (v1, v3, v1, v2, v3, v4)
        },
      };
      let bounds = AABB::new(min, max);
      place_terrain(bounds, [vtx(v1), vtx(v2), vtx(v4)]);
      place_terrain(bounds, [vtx(v2), vtx(v3), vtx(v4)]);
    };

    let platform_range = (1.0 / w) as int;
    // low platform
    for i in range_inclusive(-platform_range, platform_range) {
      for j in range_inclusive(-platform_range, platform_range) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(6.0 + i, 5.0, 0.0 + j, terrain::Dirt, Down);
        place_square(6.0 + i, 6.0, 0.0 + j, terrain::Dirt, Up);
      }
    }
    // high platform
    for i in range_inclusive(-platform_range, platform_range) {
      for j in range_inclusive(-platform_range, platform_range) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(0.0 + i, 11.0, 5.0 + j, terrain::Dirt, Down);
        place_square(0.0 + i, 12.0, 5.0 + j, terrain::Dirt, Up);
      }
    }

    let ground_range = (32.0 / w) as int;
    // ground
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(-ground_range, ground_range) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(i, 0.0, j, terrain::Grass, Up);
      }
    }

    let wall_height = (32.0 / w) as int;
    // front wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(i, j, -32.0, terrain::Stone, Back);
      }
    }
    // back wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(i, j, 32.0 - w, terrain::Stone, Front);
      }
    }
    // left wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(-32.0, j, i, terrain::Stone, Right);
      }
    }
    // right wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(32.0 - w, j, i, terrain::Stone, Left);
      }
    }
  }

  (terrains, terrain_loader)
}

fn make_mobs(
  gl: &GLContext,
  physics: &mut Physics<Id>,
  id_allocator: &mut IdAllocator,
  shader: Rc<RefCell<Shader>>,
) -> (HashMap<Id, mob::Mob>, mob::MobBuffers) {
  let mut mobs = HashMap::new();
  let mut mob_buffers = mob::MobBuffers::new(gl, shader);

  fn mob_behavior(world: &App, mob: &mut mob::Mob) {
    let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
    if Norm::norm(&to_player) < 2.0 {
      mob.behavior = wait_for_distance;
    }

    fn wait_for_distance(world: &App, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if Norm::norm(&to_player) > 8.0 {
        mob.behavior = follow_player;
      }
    }

    fn follow_player(world: &App, mob: &mut mob::Mob) {
      let mut to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if to_player.normalize() < 2.0 {
        mob.behavior = wait_to_reset;
        mob.speed = Vec3::new(0.0, 0.0, 0.0);
      } else {
        mob.speed = to_player / 2.0 as GLfloat;
      }
    }

    fn wait_to_reset(world: &App, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if Norm::norm(&to_player) >= 2.0 {
        mob.behavior = mob_behavior;
      }
    }
  }

  add_mob(
    gl,
    physics,
    &mut mobs,
    &mut mob_buffers,
    id_allocator,
    Pnt3::new(0.0, 8.0, -1.0),
    mob_behavior
  );

  (mobs, mob_buffers)
}

fn place_terrain(
  physics: &mut Physics<Id>,
  terrains: &mut HashMap<Id, terrain::TerrainPiece>,
  terrain_loader: &mut Loader<Id, Id>,
  id_allocator: &mut IdAllocator,
  bounds: AABB,
  vertices: [terrain::TerrainVertex, ..3],
  check_collisions: bool,
) {
  let mut terrain = terrain::TerrainPiece {
    vertices: vertices,
    id: Id::none(),
  };

  // hacky solution to make sure terrain polys have "breathing room" and don't
  // collide with their neighbours.
  let epsilon: GLfloat = 0.00001;
  if !check_collisions || !physics.octree.intersect(&bounds.tightened(epsilon), Id::none()) {
    terrain.id = id_allocator.allocate();
    physics.insert(terrain.id, &bounds);
    terrains.insert(terrain.id, terrain);
    terrain_loader.push(Load(terrain.id));
  }
}

fn add_mob(
  gl: &GLContext,
  physics: &mut Physics<Id>,
  mobs: &mut HashMap<Id, mob::Mob>,
  mob_buffers: &mut mob::MobBuffers,
  id_allocator: &mut IdAllocator,
  low_corner: Pnt3<GLfloat>,
  behavior: mob::Behavior,
) {
  // TODO: mob loader instead of pushing directly to gl buffers

  let id = id_allocator.allocate();

  let mob =
    mob::Mob {
      speed: Vec3::new(0.0, 0.0, 0.0),
      behavior: behavior,
      id: id,
    };

  let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as GLfloat));
  mob_buffers.push(gl, id, to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0)));

  physics.insert(id, &bounds);
  mobs.insert(id, mob);
}

/// The whole application. Wrapped up in a nice frameworky struct for piston.
pub struct App<'a> {
  physics: Physics<Id>,
  terrains: HashMap<Id, terrain::TerrainPiece>,
  player: Player,
  mobs: HashMap<Id, mob::Mob>,

  terrain_loader: Loader<Id, Id>,
  octree_loader: Rc<RefCell<Loader<(octree::OctreeId, AABB), octree::OctreeId>>>,

  // OpenGL buffers
  mob_buffers: mob::MobBuffers,
  terrain_buffers: terrain::TerrainBuffers,
  octree_buffers: octree::OctreeBuffers<Id>,
  terrain_textures: HashMap<terrain::TerrainType, Rc<Texture>>,
  line_of_sight: GLSliceBuffer<ColoredVertex>,
  hud_triangles: GLSliceBuffer<ColoredVertex>,
  text_triangles: GLSliceBuffer<TextureVertex>,

  text_textures: Vec<Texture>,

  // OpenGL shader "program" ids
  color_shader: Rc<RefCell<Shader>>,
  texture_shader: Rc<RefCell<Shader>>,
  hud_texture_shader: Rc<RefCell<Shader>>,
  hud_color_shader: Rc<RefCell<Shader>>,

  // which mouse buttons are currently pressed
  mouse_buttons_pressed: Vec<input::mouse::Button>,

  render_octree: bool,
  render_outlines: bool,

  timers: Rc<stopwatch::TimerSet>,
  gl: GLContext,
}

impl<'a> App<'a> {
  fn key_press(&mut self, key: input::keyboard::Key) {
    time!(self.timers.deref(), "event.key_press", || {
      match key {
        input::keyboard::A => {
          self.player.walk(Vec3::new(-1.0, 0.0, 0.0));
        },
        input::keyboard::D => {
          self.player.walk(Vec3::new(1.0, 0.0, 0.0));
        },
        input::keyboard::Space => {
          if !self.player.is_jumping {
            self.player.is_jumping = true;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y + 0.3;
          }
        },
        input::keyboard::W => {
          self.player.walk(Vec3::new(0.0, 0.0, -1.0));
        },
        input::keyboard::S => {
          self.player.walk(Vec3::new(0.0, 0.0, 1.0));
        },
        input::keyboard::Left =>
          self.player.rotate_lateral(PI / 12.0),
        input::keyboard::Right =>
          self.player.rotate_lateral(-PI / 12.0),
        input::keyboard::Up =>
          self.player.rotate_vertical(PI / 12.0),
        input::keyboard::Down =>
          self.player.rotate_vertical(-PI / 12.0),
        input::keyboard::M => {
          let updates = [
            ColoredVertex {
              position: self.player.camera.position,
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
            ColoredVertex {
              position: self.player.camera.position + self.player.forward() * (32.0 as f32),
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
          ];
          self.line_of_sight.update(&self.gl, 0, updates);
        },
        input::keyboard::O => {
          self.render_octree = !self.render_octree;
        }
        input::keyboard::L => {
          self.render_outlines = !self.render_outlines;
        }
        _ => {},
      }
    })
  }

  fn key_release(&mut self, key: input::keyboard::Key) {
    time!(self.timers.deref(), "event.key_release", || {
      match key {
        // accelerations are negated from those in key_press.
        input::keyboard::A => {
          self.player.walk(Vec3::new(1.0, 0.0, 0.0));
        },
        input::keyboard::D => {
          self.player.walk(Vec3::new(-1.0, 0.0, 0.0));
        },
        input::keyboard::Space => {
          if self.player.is_jumping {
            self.player.is_jumping = false;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y - 0.3;
          }
        },
        input::keyboard::W => {
          self.player.walk(Vec3::new(0.0, 0.0, 1.0));
        },
        input::keyboard::S => {
          self.player.walk(Vec3::new(0.0, 0.0, -1.0));
        },
        _ => { }
      }
    })
  }

  fn mouse_move(&mut self, w: &mut WindowSDL2, x: f64, y: f64) {
    time!(self.timers.deref(), "event.mouse_move", || {
      let (cx, cy) = (WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
      // args.y = h - args.y;
      // dy = args.y - cy;
      //  => dy = cy - args.y;
      let (dx, dy) = (x as f32 - cx, cy - y as f32);
      let (rx, ry) = (dx * -3.14 / 2048.0, dy * 3.14 / 1600.0);
      self.player.rotate_lateral(rx);
      self.player.rotate_vertical(ry);

      mouse::warp_mouse_in_window(
        &w.window,
        WINDOW_WIDTH as i32 / 2,
        WINDOW_HEIGHT as i32 / 2
      );
    })
  }

  fn mouse_press(&mut self, button: input::mouse::Button) {
    time!(self.timers.deref(), "event.mouse_press", || {
      self.mouse_buttons_pressed.push(button);
    })
  }

  fn mouse_release(&mut self, button: input::mouse::Button) {
    swap_remove_first(&mut self.mouse_buttons_pressed, button)
  }

  fn load_terrain(&mut self, max: Option<uint>) {
    time!(self.timers.deref(), "load.terrain", || {
      // terrain loading
      let count = max.map_or(self.terrain_loader.len(), |x| cmp::min(x, self.terrain_loader.len()));
      if count > 0 {
        for op in self.terrain_loader.iter(0, count) {
          let terrains = &mut self.terrains;
          let terrain_buffers = &mut self.terrain_buffers;
          let physics = &mut self.physics;
          let gl = &self.gl;
          match *op {
            Load(id) => {
              let bounds = physics.get_bounds(id).unwrap();
              let terrain = terrains.find(&id).unwrap();
              terrain_buffers.push(
                gl,
                id,
                terrain.vertices,
              );
            },
            Unload(id) => {
              if terrains.remove(&id) {
                terrain_buffers.swap_remove(gl, id);
                physics.remove(id);
              }
            },
          }
        }

        self.terrain_loader.pop(count);
      }
    });
  }

  fn load_octree(&mut self) {
    time!(self.timers.deref(), "load.octree", || {
      // octree loading
      let count = cmp::min(OCTREE_LOAD_SPEED, self.octree_loader.deref().borrow().deref().len());
      if count > 0 {
        for op in self.octree_loader.borrow().iter(0, count) {
          match *op {
            Load((id, bounds)) => {
              self.octree_buffers.push(&self.gl, id, to_outlines(&bounds));
            },
            Unload(id) => {
              self.octree_buffers.swap_remove(&self.gl, id);
            }
          }
        }

        self.octree_loader.borrow_mut().pop(count);
      }
    });
  }

  fn update(&mut self) {
    time!(self.timers.deref(), "update", || {
      // TODO(cgaebel): Ideally, the update thread should not be touching OpenGL.

      time!(self.timers.deref(), "update.load", || {
        self.load_terrain(Some(TERRAIN_LOAD_SPEED));
        self.load_octree();
      });

      time!(self.timers.deref(), "update.player", || {
        self.player.update(&mut self.physics);
      });

      time!(self.timers.deref(), "update.mobs", || {
        // Unsafely mutably borrow the mobs.
        let mobs: *mut HashMap<Id, mob:: Mob> = &mut self.mobs;
        for (_, mob) in unsafe { (*mobs).mut_iter() } {
          {
            // This code can do unsafe things with the mob vector.
            let behavior = mob.behavior;
            unsafe { (behavior)(self, mob); }
          }
          // *Safely* mutably borrow the mobs.
          // Code below here "can't" do unsafe things with the mob vector.
          let mobs = Borrow::borrow(&mut self.mobs);

          mob.speed = mob.speed - Vec3::new(0.0, 0.1, 0.0 as GLfloat);

          let delta_p = mob.speed;
          if delta_p.x != 0.0 {
            translate_mob!(self, mob, Vec3::new(delta_p.x, 0.0, 0.0));
          }
          if delta_p.y != 0.0 {
            translate_mob!(self, mob, Vec3::new(0.0, delta_p.y, 0.0));
          }
          if delta_p.z != 0.0 {
            translate_mob!(self, mob, Vec3::new(0.0, 0.0, delta_p.z));
          }
        }
      });

      // terrain deletion
      if self.is_mouse_pressed(input::mouse::Left) {
        time!(self.timers.deref(), "update.delete_terrain", || {
          for id in self.entities_in_front().into_iter() {
            if self.terrains.contains_key(&id) {
              self.remove_terrain(id);
            }
          }
        })
      }
    })
  }

  fn render(&mut self) {
    time!(self.timers.deref(), "render", || {
      self.gl.clear_buffer();

      self.color_shader.borrow_mut().set_camera(&mut self.gl, &self.player.camera);

      self.gl.use_shader(self.color_shader.borrow().deref(), |_| {
        // debug stuff
        self.line_of_sight.draw(&self.gl);

        if self.render_octree {
          self.octree_buffers.draw(&self.gl);
        }
      });

      self.texture_shader.borrow_mut().set_camera(&mut self.gl, &self.player.camera);

      // draw the world
      if self.render_outlines {
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
        gl::Disable(gl::CULL_FACE);
        self.gl.use_shader(self.texture_shader.borrow().deref(), |gl| {
          self.terrain_buffers.draw(gl);
        });
        self.gl.use_shader(self.color_shader.borrow().deref(), |gl| {
          self.mob_buffers.draw(gl);
        });
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
        gl::Enable(gl::CULL_FACE);
      } else {
        self.gl.use_shader(self.texture_shader.borrow().deref(), |gl| {
          self.terrain_buffers.draw(gl);
        });
        self.gl.use_shader(self.color_shader.borrow().deref(), |gl| {
          self.mob_buffers.draw(gl);
        });
      }

      // draw the hud
      self.gl.use_shader(self.hud_color_shader.borrow().deref(), |gl| {
        self.hud_triangles.draw(gl);
      });

      // draw hud textures
      self.gl.use_shader(self.hud_texture_shader.borrow().deref(), |gl| {
        gl::ActiveTexture(gl::TEXTURE0 + self.terrain_textures.len() as GLuint);
        for (i, tex) in self.text_textures.iter().enumerate() {
          tex.bind_2d(gl);
          self.text_triangles.draw_slice(gl, i * 2, 2);
        }
      });

      gl::Flush();
      gl::Finish();
    })
  }

  /// Initializes an empty app.
  pub fn new() -> App<'a> {
    let timers = Rc::new(stopwatch::TimerSet::new());
    time!(timers.deref(), "load", || {
      let mut gl = GLContext::new();

      gl.print_stats();

      gl::FrontFace(gl::CCW);
      gl::CullFace(gl::BACK);
      gl::Enable(gl::CULL_FACE);
      gl.enable_alpha_blending();
      gl.enable_smooth_lines();
      gl.enable_depth_buffer(100.0);
      gl.set_background_color(SKY_COLOR);
      mouse::show_cursor(false);

      let world_bounds = AABB::new(
        Pnt3 { x: -512.0, y: -32.0, z: -512.0 },
        Pnt3 { x: 512.0, y: 512.0, z: 512.0 },
      );

      let texture_shader = {
        let texture_shader =
          Rc::new(RefCell::new(shader::from_file_prefix(
            &mut gl,
            String::from_str("shaders/world_texture"),
            [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
            &FromIterator::from_iter(
              [(String::from_str("lighting"), (USE_LIGHTING as uint).to_string())].to_vec().into_iter(),
            ),
          )));
        if USE_LIGHTING {
          texture_shader.borrow_mut().set_point_light(
            &mut gl,
            &Light {
              position: Vec3::new(0.0, 16.0, 0.0),
              intensity: Vec3::new(0.6, 0.6, 0.6),
            }
          );
          texture_shader.borrow_mut().set_ambient_light(
            &mut gl,
            Vec3::new(0.4, 0.4, 0.4),
          );
          texture_shader.borrow_mut().set_ambient_light(
            &mut gl,
            Vec3::new(0.4, 0.4, 0.4),
          );
        }
        texture_shader
      };
      let color_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/color"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &HashMap::new(),
        )));
      let hud_color_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/color"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &HashMap::new(),
        )));
      let hud_texture_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/hud_texture"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &HashMap::new(),
        )));

      {
        let hud_camera = {
          let mut c = Camera::unit();
          c.fov = camera::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
          c.fov = camera::translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
          c
        };

        hud_color_shader.borrow_mut().set_camera(&mut gl, &hud_camera);
        hud_texture_shader.borrow_mut().set_camera(&mut gl, &hud_camera);
      }

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} setting up shaders", err),
      }

      let line_of_sight = {
        let mut line_of_sight = {
          GLSliceBuffer::new(
              &gl,
              color_shader.clone(),
              [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
                vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
              ],
              2,
              2,
              Lines
          )
        };

        line_of_sight.push(
          &gl,
          [
            ColoredVertex {
              position: Pnt3::new(0.0, 0.0, 0.0),
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
            ColoredVertex {
              position: Pnt3::new(0.0, 0.0, 0.0),
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
          ]
        );

        line_of_sight
      };

      let hud_triangles = make_hud(&gl, hud_color_shader.clone());

      let octree_loader = Rc::new(RefCell::new(Queue::new(1 << 20)));

      let octree_buffers = unsafe {
        octree::OctreeBuffers::new(&gl, &color_shader)
      };

      let (text_textures, text_triangles) = make_text(&gl, hud_texture_shader.clone());

      let mut physics =
        Physics {
          octree: octree::Octree::new(octree_loader.clone(), &world_bounds),
          bounds: HashMap::new(),
        };

      let mut id_allocator = IdAllocator::new();

      let (terrains, terrain_loader) =
        time!(timers.deref(), "make_terrain", || {
          make_terrain(
            &mut physics,
            &mut id_allocator,
          )
        });

      let (mobs, mob_buffers) =
        time!(timers.deref(), "make_mobs", || {
          make_mobs(
            &gl,
            &mut physics,
            &mut id_allocator,
            color_shader.clone(),
          )
        });

      let player = {
        let mut player = Player {
          camera: Camera::unit(),
          speed: Vec3::new(0.0, 0.0, 0.0),
          accel: Vec3::new(0.0, -0.1, 0.0),
          walk_accel: Vec3::new(0.0, 0.0, 0.0),
          jump_fuel: 0,
          is_jumping: false,
          id: Id::none(),
          lateral_rotation: 0.0,
          vertical_rotation: 0.0,
        };

        player.id = id_allocator.allocate();
        let min = Pnt3::new(0.0, 0.0, 0.0);
        let max = Pnt3::new(1.0, 2.0, 1.0);
        let bounds = AABB::new(min, max);
        physics.insert(player.id, &bounds);

        // initialize the projection matrix
        player.camera.translate(center(&bounds).to_vec());
        player.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);

        player.translate(&mut physics, Vec3::new(0.0, 4.0, 10.0));

        player
      };

      let terrain_textures =
        load_terrain_textures();

      let misc_texture_unit = terrain_textures.len() as GLint;
      hud_texture_shader.borrow_mut().with_uniform_location(
        &mut gl,
        "texture_in",
        |loc| {
          gl::Uniform1i(loc, misc_texture_unit);
        }
      );

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in load()", err),
      }

      debug!("load() finished with {} terrain polys", terrains.len());

      App {
        line_of_sight: line_of_sight,
        physics: physics,
        terrain_loader: terrain_loader,
        octree_loader: octree_loader,
        mob_buffers: mob_buffers,
        octree_buffers: octree_buffers,
        terrain_buffers:
          terrain::TerrainBuffers::new(
            &gl,
            texture_shader.clone(),
          ),
        terrain_textures: terrain_textures,
        terrains: terrains,
        player: player,
        mobs: mobs,
        hud_triangles: hud_triangles,
        text_textures: text_textures,
        text_triangles: text_triangles,
        color_shader: color_shader,
        texture_shader: texture_shader,
        hud_color_shader: hud_color_shader,
        hud_texture_shader: hud_texture_shader,
        mouse_buttons_pressed: Vec::new(),
        render_octree: false,
        render_outlines: false,
        timers: timers.clone(),
        gl: gl,
      }
    })
  }

  #[inline]
  pub fn is_mouse_pressed(&self, b: input::mouse::Button) -> bool {
    self.mouse_buttons_pressed.iter().any(|x| *x == b)
  }

  fn get_bounds(&self, id: Id) -> &AABB {
    self.physics.get_bounds(id).unwrap()
  }

  /// Returns ids of the closest entities in front of the cursor.
  fn entities_in_front(&self) -> Vec<Id> {
    self.physics.octree.cast_ray(&self.player.forward_ray(), self.player.id)
  }

  fn remove_terrain(&mut self, id: Id) {
    self.terrain_loader.push(Unload(id));
  }

  fn translate_mob(gl: &GLContext, physics: &mut Physics<Id>, mob_buffers: &mut mob::MobBuffers, mob: &mut mob::Mob, delta_p: Vec3<GLfloat>) {
    if physics.translate(mob.id, delta_p).unwrap() {
      mob.speed = mob.speed - delta_p;
    } else {
      let bounds = physics.get_bounds(mob.id).unwrap();
      mob_buffers.update(
        gl,
        mob.id,
        to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
      );
    }
  }

  /// Handles a game event.
  fn event(&mut self, game_window: &mut WindowSDL2, event: &mut Event) {
    match *event {
      Render(_) => self.render(),
      Update(_) => self.update(),
      Input(ref i) => match *i {
        Press(Keyboard(key)) => self.key_press(key),
        Release(Keyboard(key)) => self.key_release(key),
        Press(Mouse(button)) => self.mouse_press(button),
        Release(Mouse(button)) => self.mouse_release(button),
        Move(MouseCursor(x, y)) => self.mouse_move(game_window, x, y),
        _ => {},
      },
    }
  }

  /// Executes a game loop.
  fn run(&mut self, w: &mut WindowSDL2) {
    let mut game_iter =
      EventIterator::new(
        w,
        &EventSettings {
          updates_per_second: 30,
          max_frames_per_second: 30,
        });

    loop {
      match game_iter.next() {
        None => break,
        Some(mut e) => self.event(game_iter.window, &mut e)
      }
    }
  }
}

// TODO(cgaebel): This should be removed when rustc bug #8861 is patched.
#[unsafe_destructor]
impl<'a> Drop for App<'a> {
  fn drop(&mut self) {
    info!("Update Stats");
    info!("====================");
    self.timers.print();
  }
}

pub fn main() {
  debug!("starting");

  let mut window = WindowSDL2::new(
    OpenGL_3_3,
    WindowSettings {
      title: "playform".to_string(),
      size: [WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32],
      fullscreen: false,
      exit_on_esc: false,
      samples: 0,
    }
  );

  App::new().run(&mut window);

  debug!("finished!");
}
