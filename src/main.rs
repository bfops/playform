pub use color::Color4;
use fontloader;
use ncollide3df32::bounding_volume::aabb::AABB;
use nalgebra::na::{Vec2, Vec3, RMul, Norm};
use ncollide3df32::ray::{Ray, RayCast};
use octree;
use physics::Physics;
use piston;
use piston::*;
use gl;
use glw;
use glw::{Camera,GLfloat,Lines,Triangles,Shader,Texture,GLBuffer,GLContext,translation};
use sdl2_game_window::GameWindowSDL2;
use sdl2::mouse;
use stopwatch;
use std::cell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::iter::{range,range_inclusive};
use std::rc::Rc;
use vertex;
use vertex::{ColoredVertex, TextureVertex};

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

#[deriving(Copy, PartialEq, Eq, Hash)]
enum BlockType {
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

#[deriving(Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
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

/// A voxel-ish block in the game world.
pub struct Block {
  block_type: BlockType,
  triangles: GLBuffer<ColoredVertex>,
  outlines: GLBuffer<ColoredVertex>,
  id: Id,
}

impl Block {
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

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }

  pub fn draw_outlines(&self, gl: &GLContext) {
    self.outlines.draw(gl);
  }
}

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

type Behavior = fn(&App, &mut Mob);

pub struct Mob {
  speed: Vec3<f32>,
  behavior: Behavior,
  triangles: GLBuffer<ColoredVertex>,
  id: Id,
}

impl Mob {
  pub fn translate(&mut self, gl: &GLContext, physics: &mut Physics<Id>, v: Vec3<f32>) {
    if expect_id!(physics.translate(self.id, v)) {
      self.speed = self.speed - v;
    } else {
      let bounds = expect_id!(physics.get_bounds(self.id));
      self.triangles.update(
        gl,
        0,
        to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
      );
    }
  }

  pub fn draw(&self, gl: &GLContext) {
    self.triangles.draw(gl);
  }
}

#[inline]
pub fn swap_remove_first<T: PartialEq + Copy>(v: &mut Vec<T>, t: T) {
  match v.iter().position(|x| *x == t) {
    None => { },
    Some(i) => { v.swap_remove(i); },
  }
}

fn first_face(bounds: &AABB, ray: &Ray) -> uint {
  let f = octree::partial_min_by(
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
pub struct App {
  line_of_sight: Option<GLBuffer<ColoredVertex>>,
  physics: Physics<Id>,
  blocks: HashMap<Id, Block>,
  player: Player,
  mobs: HashMap<Id, cell::RefCell<Mob>>,
  // id of the next block to load
  next_load_id: Id,
  // next block id to assign
  next_id: Id,
  // OpenGL buffers
  hud_triangles: Option<GLBuffer<ColoredVertex>>,
  texture_triangles: Option<GLBuffer<TextureVertex>>,
  textures: Vec<Texture>,
  hud_camera: Camera,
  lateral_rotation: f32, // in radians
  vertical_rotation: f32, // in radians
  // OpenGL shader "program" id.
  color_shader: Option<Rc<Shader>>,
  texture_shader: Option<Rc<Shader>>,

  // which mouse buttons are currently pressed
  mouse_buttons_pressed: Vec<piston::mouse::Button>,

  font: fontloader::FontLoader,
  timers: stopwatch::TimerSet,
  gl: GLContext,
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
        piston::keyboard::M => {
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
          self.line_of_sight.get_mut_ref().update(&self.gl, 0, updates);
        },
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

      mouse::warp_mouse_in_window(
        &w.render_window.window,
        WINDOW_WIDTH as i32 / 2,
        WINDOW_HEIGHT as i32 / 2
      );
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
      let min = Vec3::new(0.0, 0.0, 0.0);
      let max = Vec3::new(1.0, 2.0, 1.0);
      self.physics.insert(playerId, &AABB::new(min, max));

      self.gl.enable_culling();
      self.gl.enable_alpha_blending();
      self.gl.enable_smooth_lines();
      self.gl.enable_depth_buffer();
      self.gl.set_background_color(SKY_COLOR);

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }

      self.set_up_shaders();

      // initialize the projection matrix
      self.player.camera.translate((min + max) / 2.0 as GLfloat);
      self.player.camera.fov = glw::perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in OpenGL config", err),
      }

      self.translate_player(Vec3::new(0.0, 4.0, 10.0));

      unsafe {
        self.line_of_sight = Some(GLBuffer::new(
            &self.gl,
            self.color_shader.get_ref().clone(),
            [ vertex::AttribData { name: "position", size: 3 },
              vertex::AttribData { name: "in_color", size: 4 },
            ],
            2,
            2,
            Lines
        ));

        self.hud_triangles = Some(GLBuffer::new(
            &self.gl,
            self.color_shader.get_ref().clone(),
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
            [ vertex::AttribData { name: "position", size: 3 },
              vertex::AttribData { name: "texture_position", size: 2 },
            ],
            VERTICES_PER_TRIANGLE,
            8,
            Triangles,
        ));
      }

      self.make_textures();
      self.make_hud();
      self.make_world();

      fn mob_behavior(world: &App, mob: &mut Mob) {
        let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
        if Norm::norm(&to_player) < 2.0 {
          mob.behavior = wait_for_distance;
        }

        fn wait_for_distance(world: &App, mob: &mut Mob) {
          let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
          if Norm::norm(&to_player) > 8.0 {
            mob.behavior = follow_player;
          }
        }

        fn follow_player(world: &App, mob: &mut Mob) {
          let mut to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
          if to_player.normalize() < 2.0 {
            mob.behavior = wait_to_reset;
            mob.speed = Vec3::new(0.0, 0.0, 0.0);
          } else {
            mob.speed = to_player / 2.0 as GLfloat;
          }
        }

        fn wait_to_reset(world: &App, mob: &mut Mob) {
          let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
          if Norm::norm(&to_player) >= 2.0 {
            mob.behavior = mob_behavior;
          }
        }
      }

      self.add_mob(Vec3::new(0.0, 8.0, -1.0), mob_behavior);

      self.line_of_sight.get_mut_ref().push([
        ColoredVertex {
          position: Vec3::new(0.0, 0.0, 0.0),
          color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
        },
        ColoredVertex {
          position: Vec3::new(0.0, 0.0, 0.0),
          color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
        },
      ]);
      self.line_of_sight.get_mut_ref().flush(&self.gl);
    })

    println!("load() finished with {} blocks", self.blocks.len());
  }

  fn update(&mut self, _: &mut GameWindowSDL2, _: &UpdateArgs) {
    time!(&self.timers, "update", || {
      // TODO(cgabel): Ideally, the update thread should not be touching OpenGL.

      // if there are more blocks to be loaded, add them into the OpenGL buffers.
      if self.next_load_id < self.next_id {
        time!(&self.timers, "update.load", || {
          let mut i = 0;
          while i < LOAD_SPEED && self.next_load_id < self.next_id {
            let gl = &self.gl;
            match self.blocks.find_mut(&self.next_load_id) {
              None => {},
              Some(block) => {
                let bounds = expect_id!(self.physics.get_bounds(self.next_load_id));
                block.triangles.push(to_triangles(bounds, &block.block_type.to_color()));
                block.triangles.flush(gl);
                block.outlines.push(Block::to_outlines(bounds));
                block.outlines.flush(gl);
              },
            }

            self.next_load_id = self.next_load_id + Id(1);
            i += 1;
          }
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

        // duplicated in mob update code
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

        let y_axis = Vec3::new(0.0, 1.0, 0.0);
        let walk_v =
            glw::from_axis_angle3(y_axis, self.lateral_rotation)
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

          // duplicated in player update code
          mob.speed = mob.speed - Vec3::new(0.0, 0.1, 0.0 as GLfloat);

          let dP = mob.speed;
          if dP.x != 0.0 {
            mob.translate(&self.gl, &mut self.physics, Vec3::new(dP.x, 0.0, 0.0));
          }
          if dP.y != 0.0 {
            mob.translate(&self.gl, &mut self.physics, Vec3::new(0.0, dP.y, 0.0));
          }
          if dP.z != 0.0 {
            mob.translate(&self.gl, &mut self.physics, Vec3::new(0.0, 0.0, dP.z));
          }
        }
      });

      // Block deletion
      if self.is_mouse_pressed(piston::mouse::Left) {
        time!(&self.timers, "update.delete_block", || {
          self.entity_in_front().map(|(id, _)| {
            if self.blocks.contains_key(&id) {
              self.remove_block(id);
            }
          });
        })
      }
      if self.is_mouse_pressed(piston::mouse::Right) {
        time!(&self.timers, "update.place_block", || {
          match self.entity_in_front() {
            None => {},
            Some((block_id, face)) => {
              let direction =
                    [ Vec3::new(0.0, 0.0, 1.0),
                      Vec3::new(-1.0, 0.0, 0.0),
                      Vec3::new(0.0, 1.0, 0.0),
                      Vec3::new(0.0, 0.0, -1.0),
                      Vec3::new(1.0, 0.0, 0.0),
                      Vec3::new(0.0, -1.0, 0.0),
                    ][face] * 0.5 as GLfloat;
              let (mins, maxs) = {
                let bounds = self.get_bounds(block_id);
                // TODO: think about how this should work when placing size A blocks
                // against size B blocks.
                (bounds.mins() + direction, bounds.maxs() + direction)
              };
              self.place_block(mins, maxs, Dirt, true);
            },
          }
        })
      }
    })
  }

  fn render(&mut self, _: &mut GameWindowSDL2, _: &RenderArgs) {
    time!(&self.timers, "render", || {
      self.gl.clear_buffer();

      // draw the world
      self.color_shader.get_mut_ref().set_camera(&mut self.gl, &self.player.camera);
      self.texture_shader.get_mut_ref().set_camera(&mut self.gl, &self.player.camera);

      self.line_of_sight.get_ref().draw(&self.gl);

      for (_, block) in self.blocks.iter() {
        block.draw(&self.gl);
      }

      for (_, block) in self.blocks.iter() {
        block.draw_outlines(&self.gl);
      }

      for (_, mob) in self.mobs.iter() {
        mob.borrow().draw(&self.gl);
      }

      // draw the hud
      self.color_shader.get_mut_ref().set_camera(&mut self.gl, &self.hud_camera);
      self.texture_shader.get_mut_ref().set_camera(&mut self.gl, &self.hud_camera);
      self.hud_triangles.get_ref().draw(&self.gl);

      // draw textures
      self.gl.use_shader(self.texture_shader.get_ref().deref(), |gl| {
        for (i, tex) in self.textures.iter().enumerate() {
          tex.bind_2d(gl);
          let verticies_in_a_square = 6;
          self.texture_triangles.get_ref().draw_slice(
            gl,
            i*verticies_in_a_square,
            verticies_in_a_square);
        }
      });

      gl::Flush();
      gl::Finish();
    })
  }
}

impl App {
  /// Initializes an empty app.
  pub fn new(gl: GLContext) -> App {
    let world_bounds = AABB::new(
      Vec3 { x: -512.0, y: -32.0, z: -512.0 },
      Vec3 { x: 512.0, y: 512.0, z: 512.0 },
    );
    App {
      line_of_sight: None,
      physics: Physics {
        octree: octree::Octree::new(&world_bounds),
        bounds: HashMap::new(),
        locations: HashMap::new(),
      },
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
      next_load_id: Id(1),
      // Start assigning block_ids at 1.
      // block_id 0 corresponds to no block.
      next_id: Id(1),
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
      color_shader: None,
      texture_shader: None,
      mouse_buttons_pressed: Vec::new(),
      font: fontloader::FontLoader::new(),
      timers: stopwatch::TimerSet::new(),
      gl: gl,
    }
  }

  /// Build all of our program's shaders.
  fn set_up_shaders(&mut self) {
    self.texture_shader = Some(Rc::new(Shader::new(&mut self.gl, TX_VS_SRC, TX_FS_SRC)));
    self.color_shader = Some(Rc::new(Shader::new(&mut self.gl, VS_SRC, FS_SRC)));

    match gl::GetError() {
      gl::NO_ERROR => {},
      err => fail!("OpenGL error 0x{:x} in set_up_shaders", err),
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

      self.texture_triangles.get_mut_ref().push(
        TextureVertex::square(
          Vec2 { x: -0.97, y: y - 0.2 },
          Vec2 { x: 0.0,   y: y       }
        ));
      y -= 0.2;
    }

    self.texture_triangles.get_mut_ref().flush(&self.gl);
  }

  fn make_hud(&mut self) {
    let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

    self.hud_triangles.get_mut_ref().push(
      ColoredVertex::square(
        Vec2 { x: -0.02, y: -0.02 },
        Vec2 { x:  0.02, y:  0.02 },
        cursor_color
      ));

    self.hud_triangles.get_mut_ref().flush(&self.gl);
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
    self.mouse_buttons_pressed.iter().any(|x| *x == b)
  }

  fn get_bounds(&self, id: Id) -> &AABB {
    expect_id!(self.physics.get_bounds(id))
  }

  // TODO: just return entity ID; let face be a separate call.
  /// Returns (block id, block face) shown at the center of the window.
  fn entity_in_front(&self) -> Option<(Id, uint)> {
    let ray = Ray { orig: self.player.camera.position, dir: self.forward(), };
    // TODO: fix face numbering
    self.physics.octree.cast_ray(&ray, self.player.id).map(|id| {
      let bounds = self.get_bounds(id);
      (id, first_face(bounds, &ray))
    })
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

  fn add_mob(&mut self, low_corner: Vec3<GLfloat>, behavior: fn(&App, &mut Mob)) {
    let id = self.alloc_id();

    let mut mob =
      unsafe {
        Mob {
            speed: Vec3::new(0.0, 0.0, 0.0),
            behavior: behavior,
            id: id,
            triangles: GLBuffer::new(
              &self.gl,
              self.color_shader.get_ref().clone(),
              [ vertex::AttribData { name: "position", size: 3 },
                vertex::AttribData { name: "in_color", size: 4 },
              ],
              TRIANGLE_VERTICES_PER_BOX,
              32,
              Triangles
            ),
        }
      };

    let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as GLfloat));
    mob.triangles.push(to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0)));
    mob.triangles.flush(&self.gl);

    self.physics.insert(id, &bounds);
    self.mobs.insert(id, cell::RefCell::new(mob));
  }

  fn place_block(&mut self, min: Vec3<GLfloat>, max: Vec3<GLfloat>, block_type: BlockType, check_collisions: bool) {
    time!(&self.timers, "place_block", || unsafe {
      let mut block = Block {
        block_type: block_type,
        id: Id(0),

        triangles: GLBuffer::new(
          &self.gl,
          self.color_shader.get_ref().clone(),
          [ vertex::AttribData { name: "position", size: 3 },
            vertex::AttribData { name: "in_color", size: 4 },
          ],
          TRIANGLE_VERTICES_PER_BOX,
          1,
          Triangles
        ),

        outlines: GLBuffer::new(
          &self.gl,
          self.color_shader.get_ref().clone(),
          [ vertex::AttribData { name: "position", size: 3 },
            vertex::AttribData { name: "in_color", size: 4 },
          ],
          LINE_VERTICES_PER_BOX,
          1,
          Lines
        ),
      };

      // hacky solution to make sure blocks have "breathing room" and don't
      // collide with their neighbours.
      let epsilon: GLfloat = 0.00001;
      let min = min + Vec3::new(epsilon, epsilon, epsilon);
      let max = max - Vec3::new(epsilon, epsilon, epsilon);
      let bounds = AABB::new(min, max);
      let collided = check_collisions && self.collides_with(Id(0), &bounds);

      if !collided {
        block.id = self.alloc_id();
        self.physics.insert(block.id, &bounds);
        self.blocks.insert(block.id, block);
      }
    })
  }

  fn remove_block(&mut self, id: Id) {
    self.physics.remove(id);
    self.blocks.remove(&id);
  }

  /// Changes the camera's acceleration by the given `da`.
  fn walk(&mut self, da: Vec3<GLfloat>) {
    self.player.walk_accel = self.player.walk_accel + da * 0.2 as GLfloat;
  }

  /// Translates the player/camera by a vector.
  fn translate_player(&mut self, v: Vec3<GLfloat>) {
    let id = self.player.id;
    let collided = expect_id!(self.physics.translate(id, v));
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
  pub fn forward(&self) -> Vec3<GLfloat> {
    let y_axis = Vec3::new(0.0, 1.0, 0.0);
    let transform = 
      glw::from_axis_angle3(self.right(), self.vertical_rotation) *
      glw::from_axis_angle3(y_axis, self.lateral_rotation);
    let forward_orig = Vec3::new(0.0, 0.0, -1.0);
    return transform.rmul(&forward_orig);
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

static TX_VS_SRC: &'static str =
r"#version 330 core
uniform mat4 projection_matrix;
in  vec3 position;
in  vec2 texture_position;
out vec2 tex_position;
void main() {
  tex_position = texture_position;
  gl_Position = projection_matrix * vec4(position, 1.0);
}";

static TX_FS_SRC: &'static str =
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
        max_frames_per_second: 30,
      });
  }

  println!("finished!");
  println!("");
}
