use block;
use common::*;
use fontloader;
use gl;
use gl::types::GLfloat;
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
use input;
use input::{Press,Release,Move,Keyboard,Mouse,MouseCursor};
use loader::{Loader, Load, Unload};
use mob;
use ncollide3df32::bounding_volume::LooseBoundingVolume;
use ncollide3df32::bounding_volume::aabb::AABB;
use nalgebra::na::{Vec2, Vec3, RMul, Norm};
use ncollide3df32::ray::{Ray, RayCast};
use octree;
use physics::Physics;
use piston::{WindowSettings, RenderArgs, Event, EventIterator, EventSettings, Update, Input, Render};
use png;
use sdl2_game_window::{WindowSDL2};
use sdl2::mouse;
use shader_version::opengl::*;
use stopwatch;
use stopwatch::*;
use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::iter::{range,range_inclusive};
use std::mem;
use std::raw;
use std::rc::Rc;
use libc::types::common::c95::c_void;

static MAX_JUMP_FUEL: uint = 4;

// how many blocks to load during every update step
static BLOCK_LOAD_SPEED:uint = 1 << 9;
static OCTREE_LOAD_SPEED:uint = 1 << 11;
static SKY_COLOR: Color4<GLfloat>  = Color4 {r: 0.2, g: 0.5, b: 0.7, a: 1.0 };

#[deriving(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
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

fn to_faces(bounds: &AABB) -> [AABB, ..6] {
  let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
  let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

  [
    AABB::new(Vec3::new(x1, y1, z2), Vec3::new(x2, y2, z2)),
    AABB::new(Vec3::new(x1, y1, z1), Vec3::new(x1, y2, z2)),
    AABB::new(Vec3::new(x1, y2, z1), Vec3::new(x2, y2, z2)),
    AABB::new(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z1)),
    AABB::new(Vec3::new(x2, y1, z1), Vec3::new(x2, y2, z2)),
    AABB::new(Vec3::new(x1, y1, z1), Vec3::new(x2, y1, z2)),
  ]
}

fn to_triangles(bounds: &AABB, c: &Color4<GLfloat>) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BOX] {
  let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
  let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

  let vtx = |x, y, z| {
    ColoredVertex {
      position: Vec3::new(x, y, z),
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

fn center(bounds: &AABB) -> Vec3<GLfloat> {
  (bounds.mins() + *bounds.maxs()) / (2.0 as GLfloat)
}

pub struct Player {
  camera: Camera,
  // speed; units are world coordinates
  speed: Vec3<GLfloat>,
  // acceleration; units are world coordinates
  accel: Vec3<GLfloat>,
  // acceleration; x/z units are relative to player facing
  walk_accel: Vec3<GLfloat>,
  // this is depleted as we jump and replenished as we stand.
  jump_fuel: uint,
  // are we currently trying to jump? (e.g. holding the key).
  is_jumping: bool,
  id: Id,
}

#[inline]
pub fn swap_remove_first<T: PartialEq + Copy>(v: &mut Vec<T>, t: T) {
  match v.iter().position(|x| *x == t) {
    None => { },
    Some(i) => { v.swap_remove(i); },
  }
}

fn first_face(bounds: &AABB, ray: &Ray) -> uint {
  let f = partial_min_by(
      to_faces(bounds)
        .iter()
        .zip(range(0 as uint, 6))
        .filter_map(|(bounds, i)| {
            bounds.toi_with_ray(ray, true).map(|x| (x, i))
          }),
      |(toi, _)| toi
    )
    .map(|(_, i)| i)
    .expect("ray does not intersect any faces");
  f
}

/// The whole application. Wrapped up in a nice frameworky struct for piston.
pub struct App<'a> {
  physics: Physics<Id>,
  blocks: HashMap<Id, block::Block>,
  player: Player,
  mobs: HashMap<Id, RefCell<mob::Mob>>,

  // next block id to assign
  next_id: Id,

  block_loader: Loader<Id, Id>,
  octree_loader: Rc<RefCell<Loader<(octree::OctreeId, AABB), octree::OctreeId>>>,

  // OpenGL buffers
  mob_buffers: mob::MobBuffers,
  block_buffers: HashMap<block::BlockType, block::BlockBuffers>,
  octree_buffers: octree::OctreeBuffers<Id>,
  block_textures: HashMap<block::BlockType, Rc<Texture>>,
  line_of_sight: GLSliceBuffer<ColoredVertex>,
  hud_triangles: GLSliceBuffer<ColoredVertex>,
  texture_triangles: GLSliceBuffer<TextureVertex>,

  textures: Vec<Texture>,
  hud_camera: Camera,
  lateral_rotation: f32, // in radians
  vertical_rotation: f32, // in radians

  // OpenGL shader "program" ids
  color_shader: Rc<RefCell<Shader>>,
  hud_shader: Rc<RefCell<Shader>>,
  texture_shader: Rc<RefCell<Shader>>,

  // which mouse buttons are currently pressed
  mouse_buttons_pressed: Vec<input::mouse::Button>,

  render_octree: bool,
  render_outlines: bool,

  font: fontloader::FontLoader,
  timers: stopwatch::TimerSet,
  gl: GLContext,
}

impl<'a> App<'a> {
  fn key_press(&mut self, key: input::keyboard::Key) {
    time!(&self.timers, "event.key_press", || {
      match key {
        input::keyboard::A => {
          self.walk(Vec3::new(-1.0, 0.0, 0.0));
        },
        input::keyboard::D => {
          self.walk(Vec3::new(1.0, 0.0, 0.0));
        },
        input::keyboard::Space => {
          if !self.player.is_jumping {
            self.player.is_jumping = true;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y + 0.3;
          }
        },
        input::keyboard::W => {
          self.walk(Vec3::new(0.0, 0.0, -1.0));
        },
        input::keyboard::S => {
          self.walk(Vec3::new(0.0, 0.0, 1.0));
        },
        input::keyboard::Left =>
          self.rotate_lateral(PI / 12.0),
        input::keyboard::Right =>
          self.rotate_lateral(-PI / 12.0),
        input::keyboard::Up =>
          self.rotate_vertical(PI / 12.0),
        input::keyboard::Down =>
          self.rotate_vertical(-PI / 12.0),
        input::keyboard::M => {
          let updates = [
            ColoredVertex {
              position: self.player.camera.position,
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
            ColoredVertex {
              position: self.player.camera.position + self.forward() * (32.0 as f32),
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
    time!(&self.timers, "event.key_release", || {
      match key {
        // accelerations are negated from those in key_press.
        input::keyboard::A => {
          self.walk(Vec3::new(1.0, 0.0, 0.0));
        },
        input::keyboard::D => {
          self.walk(Vec3::new(-1.0, 0.0, 0.0));
        },
        input::keyboard::Space => {
          if self.player.is_jumping {
            self.player.is_jumping = false;
            // this 0.3 is duplicated in a few places
            self.player.accel.y = self.player.accel.y - 0.3;
          }
        },
        input::keyboard::W => {
          self.walk(Vec3::new(0.0, 0.0, 1.0));
        },
        input::keyboard::S => {
          self.walk(Vec3::new(0.0, 0.0, -1.0));
        },
        _ => { }
      }
    })
  }

  fn mouse_move(&mut self, w: &mut WindowSDL2, x: f64, y: f64) {
    time!(&self.timers, "event.mouse_move", || {
      let (cx, cy) = (WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
      // args.y = h - args.y;
      // dy = args.y - cy;
      //  => dy = cy - args.y;
      let (dx, dy) = (x as f32 - cx, cy - y as f32);
      let (rx, ry) = (dx * -3.14 / 2048.0, dy * 3.14 / 1600.0);
      self.rotate_lateral(rx);
      self.rotate_vertical(ry);

      mouse::warp_mouse_in_window(
        &w.window,
        WINDOW_WIDTH as i32 / 2,
        WINDOW_HEIGHT as i32 / 2
      );
    })
  }

  fn mouse_press(&mut self, button: input::mouse::Button) {
    time!(&self.timers, "event.mouse_press", || {
      self.mouse_buttons_pressed.push(button);
    })
  }

  fn mouse_release(&mut self, button: input::mouse::Button) {
    swap_remove_first(&mut self.mouse_buttons_pressed, button)
  }

  fn load(&mut self) {
    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in initialization", err),
    }

    time!(&self.timers, "load", || {
      mouse::show_cursor(false);

      let player_id = self.alloc_id();
      self.player.id = player_id;
      let min = Vec3::new(0.0, 0.0, 0.0);
      let max = Vec3::new(1.0, 2.0, 1.0);
      self.physics.insert(player_id, &AABB::new(min, max));

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }

      self.load_block_textures();

      // initialize the projection matrix
      self.player.camera.translate((min + max) / 2.0 as GLfloat);
      self.player.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }

      self.translate_player(Vec3::new(0.0, 4.0, 10.0));

      self.make_textures();
      self.make_hud();
      self.make_world();

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

      self.add_mob(Vec3::new(0.0, 8.0, -1.0), mob_behavior);

      self.line_of_sight.push(
        &self.gl,
        [
          ColoredVertex {
            position: Vec3::new(0.0, 0.0, 0.0),
            color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
          },
          ColoredVertex {
            position: Vec3::new(0.0, 0.0, 0.0),
            color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
          },
        ]
      );
    })

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in load()", err),
    }

    println!("load() finished with {} blocks", self.blocks.len());
  }

  fn load_blocks(&mut self, max: Option<uint>) {
    time!(&self.timers, "load.blocks", || {
      // block loading
      let count = max.map_or(self.block_loader.len(), |x| cmp::min(x, self.block_loader.len()));
      if count > 0 {
        for op in self.block_loader.iter(0, count) {
          let blocks = &mut self.blocks;
          let block_buffers = &mut self.block_buffers;
          let physics = &mut self.physics;
          let gl = &self.gl;
          match *op {
            Load(id) => {
              let bounds = unwrap!(physics.get_bounds(id));
              let block = unwrap!(blocks.find(&id));
              block_buffers.find_mut(&block.block_type).unwrap().push(
                gl,
                id,
                block::Block::to_texture_triangles(bounds),
                block::Block::to_outlines(bounds)
              );
            },
            Unload(id) => {
              {
                let block = unwrap!(blocks.find(&id));
                let block_type = block.block_type;
                let buffer = unwrap!(block_buffers.find_mut(&block_type));
                buffer.swap_remove(gl, id);
              }
              physics.remove(id);
              blocks.remove(&id);
            },
          }
        }

        self.block_loader.pop(count);
      }
    });
  }

  fn load_octree(&mut self) {
    time!(&self.timers, "load.octree", || {
      // octree loading
      let count = cmp::min(OCTREE_LOAD_SPEED, self.octree_loader.deref().borrow().deref().len());
      if count > 0 {
        for op in self.octree_loader.deref().borrow().deref().iter(0, count) {
          match *op {
            Load((id, bounds)) => {
              self.octree_buffers.push(&self.gl, id, to_outlines(&bounds));
            },
            Unload(id) => {
              self.octree_buffers.swap_remove(&self.gl, id);
            }
          }
        }

        self.octree_loader.deref().borrow_mut().deref_mut().pop(count);
      }
    });
  }

  fn update(&mut self) {
    time!(&self.timers, "update", || {
      // TODO(cgaebel): Ideally, the update thread should not be touching OpenGL.

      time!(&self.timers, "update.load", || {
        self.load_blocks(Some(BLOCK_LOAD_SPEED));
        self.load_octree();
      });

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

        let delta_p = self.player.speed;
        if delta_p.x != 0.0 {
          self.translate_player(Vec3::new(delta_p.x, 0.0, 0.0));
        }
        if delta_p.y != 0.0 {
          self.translate_player(Vec3::new(0.0, delta_p.y, 0.0));
        }
        if delta_p.z != 0.0 {
          self.translate_player(Vec3::new(0.0, 0.0, delta_p.z));
        }

        let y_axis = Vec3::new(0.0, 1.0, 0.0);
        let walk_v =
            camera::from_axis_angle3(y_axis, self.lateral_rotation)
            .rmul(&self.player.walk_accel);
        self.player.speed = self.player.speed + walk_v + self.player.accel;
        // friction
        self.player.speed = self.player.speed * Vec3::new(0.7, 0.99, 0.7 as f32);
      });

      time!(&self.timers, "update.mobs", || {
        for (&id, mob) in self.mobs.iter() {
          {
            let behavior = mob.borrow().behavior.clone();
            let mut mob = mob.borrow_mut();
            (behavior)(self, mob.deref_mut());
          }
          let mut mob = mob.borrow_mut();
          let mob = mob.deref_mut();

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

      // block::Block deletion
      if self.is_mouse_pressed(input::mouse::Left) {
        time!(&self.timers, "update.delete_block", || {
          self.entity_in_front().map(|id| {
            if self.blocks.contains_key(&id) {
              self.remove_block(id);
            }
          });
        })
      }
      if self.is_mouse_pressed(input::mouse::Right) {
        time!(&self.timers, "update.place_block", || {
          match self.entity_in_front() {
            None => {},
            Some(block_id) => {
              let (mins, maxs) = {
                let bounds = self.get_bounds(block_id);
                let face = first_face(bounds, &self.forward_ray());
                let direction =
                      [ Vec3::new(0.0, 0.0, 1.0),
                        Vec3::new(-1.0, 0.0, 0.0),
                        Vec3::new(0.0, 1.0, 0.0),
                        Vec3::new(0.0, 0.0, -1.0),
                        Vec3::new(1.0, 0.0, 0.0),
                        Vec3::new(0.0, -1.0, 0.0),
                      ][face] * 0.5 as GLfloat;
                // TODO: think about how this should work when placing size A blocks
                // against size B blocks.
                (bounds.mins() + direction, bounds.maxs() + direction)
              };
              self.place_block(mins, maxs, block::Dirt, true);
            },
          }
        })
      }
    })
  }

  fn render(&mut self, _: &mut WindowSDL2, _: &RenderArgs) {
    time!(&self.timers, "render", || {
      self.gl.clear_buffer();

      // draw the world

      self.color_shader.borrow_mut().set_camera(&mut self.gl, &self.player.camera);
      self.texture_shader.borrow_mut().set_camera(&mut self.gl, &self.player.camera);

      // debug stuff
      self.line_of_sight.draw(&self.gl);

      if self.render_octree {
        self.octree_buffers.draw(&self.gl);
      }

      // draw the blocks
      for (block_type, buffers) in self.block_buffers.iter() {
        let r = unwrap!(self.block_textures.find(block_type));
        r.bind_2d(&self.gl);
        buffers.draw(&self.gl, self.render_outlines);
      }

      self.mob_buffers.draw(&self.gl);

      // draw the hud

      self.color_shader.borrow_mut().set_camera(&mut self.gl, &self.hud_camera);
      self.hud_triangles.draw(&self.gl);

      // draw hud textures
      self.gl.use_shader(self.hud_shader.borrow().deref(), |gl| {
        for (i, tex) in self.textures.iter().enumerate() {
          tex.bind_2d(gl);
          self.texture_triangles.draw_slice(gl, i * 2, 2);
        }
      });

      gl::Flush();
      gl::Finish();
    })
  }

  /// Initializes an empty app.
  pub fn new(gl: GLContext) -> App<'a> {
    let mut gl = gl;

    gl.enable_culling();
    gl.enable_alpha_blending();
    gl.enable_smooth_lines();
    gl.enable_depth_buffer(100.0);
    gl.set_background_color(SKY_COLOR);

    let world_bounds = AABB::new(
      Vec3 { x: -512.0, y: -32.0, z: -512.0 },
      Vec3 { x: 512.0, y: 512.0, z: 512.0 },
    );

    let texture_shader = {
      let texture_shader =
        Rc::new(RefCell::new(Shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/world_texture"),
          Vec::from_slice([ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ]).move_iter(),
        )));
      texture_shader.borrow_mut().deref_mut().set_point_light(
        &mut gl,
        &Light {
          position: Vec3::new(0.0, 16.0, 0.0),
          intensity: Vec3::new(0.6, 0.6, 0.6),
        }
      );
      texture_shader.borrow_mut().deref_mut().set_ambient_light(
        &mut gl,
        Vec3::new(0.4, 0.4, 0.4),
      );
      texture_shader
    };
    let color_shader =
      Rc::new(RefCell::new(Shader::from_file_prefix(
        &mut gl,
        String::from_str("shaders/world_color"),
        Vec::from_slice([ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ]).move_iter(),
      )));
    let hud_shader =
      Rc::new(RefCell::new(Shader::from_file_prefix(
        &mut gl,
        String::from_str("shaders/hud_texture"),
        Vec::from_slice([ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ]).move_iter(),
      )));

    let hud_camera = {
      let mut c = Camera::unit();
      c.fov = camera::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
      c.fov = camera::translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
      c
    };

    hud_shader.borrow_mut().deref_mut().set_camera(&mut gl, &hud_camera);

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} setting up shaders", err),
    }

    let line_of_sight = {
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

    let hud_triangles = {
      GLSliceBuffer::new(
          &gl,
          color_shader.clone(),
          [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
            vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
          ],
          VERTICES_PER_TRIANGLE,
          16,
          Triangles
      )
    };

    let texture_triangles = {
      GLSliceBuffer::new(
          &gl,
          hud_shader.clone(),
          [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
            vertex::AttribData { name: "texture_position", size: 2, unit: vertex::Float },
          ],
          VERTICES_PER_TRIANGLE,
          8,
          Triangles,
      )
    };

    let mob_buffers = unsafe {
      mob::MobBuffers::new(&gl, &color_shader)
    };

    let octree_loader = Rc::new(RefCell::new(Queue::new(1 << 20)));

    let octree_buffers = unsafe {
      octree::OctreeBuffers::new(&gl, &color_shader)
    };

    App {
      line_of_sight: line_of_sight,
      physics: Physics {
        octree: octree::Octree::new(&octree_loader, &world_bounds),
        bounds: HashMap::new(),
        locations: HashMap::new(),
      },
      block_loader: Queue::new(1 << 20),
      octree_loader: octree_loader,
      mob_buffers: mob_buffers,
      octree_buffers: octree_buffers,
      block_buffers: HashMap::new(),
      block_textures: HashMap::new(),
      blocks: HashMap::new(),
      player: Player {
        camera: Camera::unit(),
        speed: Vec3::new(0.0, 0.0, 0.0),
        accel: Vec3::new(0.0, -0.1, 0.0),
        walk_accel: Vec3::new(0.0, 0.0, 0.0),
        jump_fuel: 0,
        is_jumping: false,
        id: Id(0),
      },
      mobs: HashMap::new(),
      // Start assigning block_ids at 1.
      // block_id 0 corresponds to no block.
      next_id: Id(1),
      hud_triangles: hud_triangles,
      texture_triangles: texture_triangles,
      textures: Vec::new(),
      hud_camera: hud_camera,
      lateral_rotation: 0.0,
      vertical_rotation: 0.0,
      color_shader: color_shader,
      texture_shader: texture_shader,
      hud_shader: hud_shader,
      mouse_buttons_pressed: Vec::new(),
      render_octree: false,
      render_outlines: true,
      font: fontloader::FontLoader::new(),
      timers: stopwatch::TimerSet::new(),
      gl: gl,
    }
  }

  fn load_block_textures(&mut self) {
    for &(block_type, path) in [(block::Grass, "textures/grass.png"), (block::Stone, "textures/stone.png"), (block::Dirt, "textures/dirt.png")].iter() {
      let img = match png::load_png(&Path::new(path)) {
        Ok(i) => i,
        Err(s) => fail!("Could not load png {}: {}", path, s)
      };
      if img.color_type != png::RGBA8 {
        fail!("unsupported color type {:} in png", img.color_type);
      }
      println!("loaded rgba8 png file {}", path);

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

      self.block_textures.insert(block_type, Rc::new(Texture { id: texture }));
      self.block_buffers.insert(
        block_type,
        block::BlockBuffers::new(
          &self.gl,
          &self.color_shader,
          &self.texture_shader,
        )
      );
    }
  }

  /// Makes some basic textures in the world.
  fn make_textures(&mut self) {
    let instructions = Vec::from_slice([
            "Use WASD to move, and spacebar to jump.",
            "Use the mouse to look around, and click to remove blocks."
        ]);

    let mut y = 0.99;

    for line in instructions.iter() {
      self.textures.push(self.font.sans.red(*line));

      self.texture_triangles.push(
        &self.gl,
        TextureVertex::square(
          Vec2 { x: -0.97, y: y - 0.2 },
          Vec2 { x: 0.0,   y: y       }
        )
      );
      y -= 0.2;
    }
  }

  fn make_hud(&mut self) {
    let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

    self.hud_triangles.push(
      &self.gl,
      ColoredVertex::square(
        Vec2 { x: -0.02, y: -0.02 },
        Vec2 { x:  0.02, y:  0.02 },
        cursor_color
      )
    );
  }

  fn make_world(&mut self) {
    time!(&self.timers, "make_world", || {
      // low dirt block
      for i in range_inclusive(-2i, 2) {
        for j in range_inclusive(-2i, 2) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (6.0 + i, 6.0, 0.0 + j);
          let (x2, y2, z2) = (6.5 + i, 6.5, 0.5 + j);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Dirt, false);
        }
      }
      // high dirt block
      for i in range_inclusive(-2i, 2) {
        for j in range_inclusive(-2i, 2) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (0.0 + i, 12.0, 5.0 + j);
          let (x2, y2, z2) = (0.5 + i, 12.5, 5.5 + j);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Dirt, false);
        }
      }
      // ground
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(-64i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (i, 0.0, j);
          let (x2, y2, z2) = (i + 0.5, 0.5, j + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Grass, false);
        }
      }
      // front wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (i, 0.5 + j, -32.0);
          let (x2, y2, z2) = (i + 0.5, 1.0 + j, -32.0 + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Stone, false);
        }
      }
      // back wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (i, 0.5 + j, 32.0);
          let (x2, y2, z2) = (i + 0.5, 1.0 + j, 32.0 + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Stone, false);
        }
      }
      // left wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (-32.0, 0.5 + j, i);
          let (x2, y2, z2) = (-32.0 + 0.5, 1.0 + j, i + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Stone, false);
        }
      }
      // right wall
      for i in range_inclusive(-64i, 64) {
        for j in range_inclusive(0i, 64) {
          let (i, j) = (i as GLfloat / 2.0, j as GLfloat / 2.0);
          let (x1, y1, z1) = (32.0, 0.5 + j, i);
          let (x2, y2, z2) = (32.0 + 0.5, 1.0 + j, i + 0.5);
          self.place_block(Vec3::new(x1, y1, z1), Vec3::new(x2, y2, z2), block::Stone, false);
        }
      }
    });
  }

  #[inline]
  pub fn is_mouse_pressed(&self, b: input::mouse::Button) -> bool {
    self.mouse_buttons_pressed.iter().any(|x| *x == b)
  }

  fn get_bounds(&self, id: Id) -> &AABB {
    unwrap!(self.physics.get_bounds(id))
  }

  /// Returns id of the entity in front of the cursor.
  fn entity_in_front(&self) -> Option<Id> {
    self.physics.octree.cast_ray(&self.forward_ray(), self.player.id)
  }

  /// Find a collision with a world object
  fn collides_with(&self, self_id: Id, b: &AABB) -> bool {
    time!(&self.timers, "world_collision", || {
      self.physics.octree.intersect(b, self_id)
    })
  }

  fn alloc_id(&mut self) -> Id {
    let id = self.next_id;
    self.next_id = self.next_id + Id(1);
    id
  }

  fn add_mob(&mut self, low_corner: Vec3<GLfloat>, behavior: fn(&App, &mut mob::Mob)) {
    // TODO: mob loader instead of pushing directly to gl buffers

    let id = self.alloc_id();

    let mob =
      mob::Mob {
        speed: Vec3::new(0.0, 0.0, 0.0),
        behavior: behavior,
        id: id,
      };

    let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as GLfloat));
    self.mob_buffers.push(&self.gl, id, to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0)));

    self.physics.insert(id, &bounds);
    self.mobs.insert(id, RefCell::new(mob));
  }

  fn place_block(&mut self, min: Vec3<GLfloat>, max: Vec3<GLfloat>, block_type: block::BlockType, check_collisions: bool) {
    time!(&self.timers, "place_block", || {
      let mut block = block::Block {
        block_type: block_type,
        id: Id(0),
      };

      // hacky solution to make sure blocks have "breathing room" and don't
      // collide with their neighbours.
      let epsilon: GLfloat = 0.00001;
      let bounds = AABB::new(min, max).loosened(-epsilon);
      let collided = check_collisions && self.collides_with(Id(0), &bounds);

      if !collided {
        block.id = self.alloc_id();
        self.blocks.insert(block.id, block);
        self.physics.insert(block.id, &bounds);
        self.block_loader.push(Load(block.id));
      }
    })
  }

  fn remove_block(&mut self, id: Id) {
    self.block_loader.push(Unload(id));
  }

  /// Changes the camera's acceleration by the given `da`.
  fn walk(&mut self, da: Vec3<GLfloat>) {
    self.player.walk_accel = self.player.walk_accel + da * 0.2 as GLfloat;
  }

  /// Translates the player/camera by a vector.
  fn translate_player(&mut self, v: Vec3<GLfloat>) {
    let id = self.player.id;
    let collided = unwrap!(self.physics.translate(id, v));
    if collided {
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

  fn translate_mob(gl: &GLContext, physics: &mut Physics<Id>, mob_buffers: &mut mob::MobBuffers, mob: &mut mob::Mob, delta_p: Vec3<GLfloat>) {
    if unwrap!(physics.translate(mob.id, delta_p)) {
      mob.speed = mob.speed - delta_p;
    } else {
      let bounds = unwrap!(physics.get_bounds(mob.id));
      mob_buffers.update(
        gl,
        mob.id,
        to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
      );
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
      camera::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation)
        .rmul(&Vec3::new(1.0, 0.0, 0.0))
  }

  /// Return the "Ray axis (i.e. the z-axis rotated to match you).
  pub fn forward(&self) -> Vec3<GLfloat> {
    let y_axis = Vec3::new(0.0, 1.0, 0.0);
    let transform =
      camera::from_axis_angle3(self.right(), self.vertical_rotation) *
      camera::from_axis_angle3(y_axis, self.lateral_rotation);
    let forward_orig = Vec3::new(0.0, 0.0, -1.0);
    return transform.rmul(&forward_orig);
  }

  pub fn forward_ray(&self) -> Ray {
    Ray { orig: self.player.camera.position, dir: self.forward() }
  }

  /// Handles a game event.
  fn event(&mut self, game_window: &mut WindowSDL2, event: &mut Event) {
    match *event {
      Render(ref mut args) => self.render(game_window, args),
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
    self.load();

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
    println!("Update Stats");
    println!("====================");
    self.timers.print();
    println!("");
  }
}

pub fn main() {
  println!("starting");

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

  let gl = GLContext::new();

  gl.print_stats();

  App::new(gl).run(&mut window);

  println!("finished!");
  println!("");
}
