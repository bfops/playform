use camera;
use color::{Color3, Color4};
use common::*;
use fontloader;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use interval_timer::IntervalTimer;
use light::{Light, set_point_light, set_ambient_light};
use lod_map::{LOD, OwnerId};
use mob;
use nalgebra::{Pnt2, Vec2, Vec3, Pnt3, Norm};
use nalgebra;
use ncollide::bounding_volume::{AABB, AABB3};
use physics::Physics;
use player::Player;
use shaders;
use stopwatch::TimerSet;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::f32::consts::PI;
use std::ops::Add;
use std::rc::Rc;
use surroundings_loader::SurroundingsLoader;
use terrain;
use terrain_game_loader::TerrainGameLoader;
use terrain_vram_buffers;
use time;
use vertex::{ColoredVertex, TextureVertex};
use yaglw::vertex_buffer::*;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::{Texture2D, TextureUnit};

const SUN_TICK_NS: u64 = 1000000;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
pub struct EntityId(u32);

impl Default for EntityId {
  fn default() -> EntityId {
    EntityId(0)
  }
}

impl Add<u32> for EntityId {
  type Output = EntityId;

  fn add(self, rhs: u32) -> EntityId {
    let EntityId(i) = self;
    EntityId(i + rhs)
  }
}

fn center(bounds: &AABB3<f32>) -> Pnt3<GLfloat> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as GLfloat)
}

fn make_text<'a>(
  gl: &'a GLContextExistence,
  gl_context: &mut GLContext,
  shader: &Shader<'a>,
) -> (Vec<Texture2D<'a>>, GLArray<'a, TextureVertex>) {
  let fontloader = fontloader::FontLoader::new();
  let mut textures = Vec::new();
  let buffer = GLBuffer::new(gl, gl_context, 8 * VERTICES_PER_TRIANGLE as usize);
  let mut triangles =
    GLArray::new(
      gl,
      gl_context,
      shader,
      &[
        VertexAttribData { name: "position", size: 3, unit: GLType::Float },
        VertexAttribData { name: "texture_position", size: 2, unit: GLType::Float },
      ],
      DrawMode::Triangles,
      buffer,
    );

  let instructions =
    &[
      "Use WASD to move, and spacebar to jump.",
      "Use the mouse to look around.",
    ].to_vec();

  let mut y = 0.99;

  for line in instructions.iter() {
    textures.push(fontloader.sans.red(gl, *line));

    triangles.push(
      gl_context,
      &TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }

  (textures, triangles)
}

fn make_hud<'a>(
  gl: &'a GLContextExistence,
  gl_context: &mut GLContext,
  shader: &Shader<'a>,
) -> GLArray<'a ,ColoredVertex> {
  let buffer = GLBuffer::new(gl, gl_context, 16 * VERTICES_PER_TRIANGLE as usize);
  let mut hud_triangles = {
    GLArray::new(
      gl,
      gl_context,
      shader,
      &[
        VertexAttribData { name: "position", size: 3, unit: GLType::Float },
        VertexAttribData { name: "in_color", size: 4, unit: GLType::Float },
      ],
      DrawMode::Triangles,
      buffer,
    )
  };

  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  hud_triangles.push(
    gl_context,
    &ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    )
  );

  hud_triangles
}

/// The whole application. Wrapped up in a nice frameworky struct for piston.
pub struct App<'a> {
  pub physics: Physics,
  pub player: Player<'a>,
  pub mobs: HashMap<EntityId, Rc<RefCell<mob::Mob<'a>>>>,
  // The sun as portions of a 65536-degree circle.
  pub sun: u16,
  pub sun_timer: IntervalTimer,

  pub id_allocator: IdAllocator<EntityId>,
  pub terrain_game_loader: TerrainGameLoader<'a>,

  // OpenGL buffers
  pub mob_buffers: mob::MobBuffers<'a>,
  pub line_of_sight: GLArray<'a, ColoredVertex>,
  pub hud_triangles: GLArray<'a, ColoredVertex>,
  pub text_triangles: GLArray<'a, TextureVertex>,

  pub misc_texture_unit: TextureUnit,
  pub text_textures: Vec<Texture2D<'a>>,

  // OpenGL shader "program" ids
  pub mob_shader: shaders::color::ColorShader<'a>,
  pub terrain_shader: shaders::terrain::TerrainShader<'a>,
  pub hud_texture_shader: shaders::texture::TextureShader<'a>,
  pub hud_color_shader: shaders::color::ColorShader<'a>,

  pub render_outlines: bool,

  pub timers: &'a TimerSet,
  pub gl: &'a GLContextExistence,
  pub gl_context: &'a mut GLContext,
}

impl<'a> App<'a> {
  /// Initializes an empty app.
  pub fn new(
    gl: &'a GLContextExistence,
    gl_context: &'a mut GLContext,
    timers: &'a TimerSet,
  ) -> App<'a> {
    gl_context.print_stats();

    unsafe {
      gl::FrontFace(gl::CCW);
      gl::CullFace(gl::BACK);
      gl::Enable(gl::CULL_FACE);
    }
    gl_context.enable_alpha_blending();
    gl_context.enable_smooth_lines();
    gl_context.enable_depth_buffer(1.0);

    let mut terrain_shader = {
      let mut terrain_shader = shaders::terrain::TerrainShader::new(gl);
      set_point_light(
        &mut terrain_shader.shader,
        gl_context,
        &Light {
          position: Pnt3::new(0.0, 0.0, 0.0),
          intensity: Color3::of_rgb(0.0, 0.0, 0.0),
        }
      );
      set_ambient_light(
        &mut terrain_shader.shader,
        gl_context,
        Color3::of_rgb(0.4, 0.4, 0.4),
      );
      terrain_shader
    };
    let mob_shader = shaders::color::ColorShader::new(gl);
    let mut hud_color_shader = shaders::color::ColorShader::new(gl);
    let mut hud_texture_shader = shaders::texture::TextureShader::new(gl);

    {
      let hud_camera = {
        let mut c = camera::Camera::unit();
        c.fov = camera::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
        c.fov = camera::translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
        c
      };

      camera::set_camera(
        &mut hud_color_shader.shader,
        gl_context,
        &hud_camera,
      );
      camera::set_camera(
        &mut hud_texture_shader.shader,
        gl_context,
        &hud_camera,
      );
    }

    match gl_context.get_error() {
      gl::NO_ERROR => {},
      err => warn!("OpenGL error 0x{:x} setting up shaders", err),
    }

    let line_of_sight = {
      let buffer = GLBuffer::new(gl, gl_context, 2 * 2);
      let mut line_of_sight = {
        GLArray::new(
          gl,
          gl_context,
          &mob_shader.shader,
          &[
	          VertexAttribData { name: "position", size: 3, unit: GLType::Float },
            VertexAttribData { name: "in_color", size: 4, unit: GLType::Float },
          ],
          DrawMode::Lines,
          buffer,
        )
      };

      line_of_sight.push(
        gl_context,
        &[
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

    let hud_triangles = make_hud(gl, gl_context, &hud_color_shader.shader);

    let mut texture_unit_alloc: IdAllocator<TextureUnit> = IdAllocator::new();
    let terrain_game_loader =
      TerrainGameLoader::new(gl, gl_context, &mut terrain_shader, &mut texture_unit_alloc);

    let (text_textures, text_triangles) =
      make_text(gl, gl_context, &hud_texture_shader.shader);

    let world_width: u32 = 1 << 11;
    let world_width = world_width as f32;
    let mut physics =
      Physics::new(
        AABB::new(
          Pnt3 { x: -world_width, y: -2.0 * terrain::AMPLITUDE as f32, z: -world_width },
          Pnt3 { x: world_width, y: 2.0 * terrain::AMPLITUDE as f32, z: world_width },
        )
      );

    let mut id_allocator = IdAllocator::new();
    let mut owner_allocator = IdAllocator::new();

    let (mobs, mob_buffers) =
      timers.time("make_mobs", || {
        make_mobs(
          gl,
          gl_context,
          &mut physics,
          &mut id_allocator,
          &mut owner_allocator,
          &mob_shader,
        )
      });

    let player = {
      let mut load_distance = Player::load_distance(terrain_vram_buffers::POLYGON_BUDGET as i32);

      // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
      let max_load_distance = 60;
      if load_distance > max_load_distance {
        info!("load_distance {} capped at {}", load_distance, max_load_distance);
        load_distance = max_load_distance;
      } else {
        info!("load_distance {}", load_distance);
      }

      let mut player = Player {
        camera: camera::Camera::unit(),
        speed: Vec3::new(0.0, 0.0, 0.0),
        accel: Vec3::new(0.0, -0.1, 0.0),
        walk_accel: Vec3::new(0.0, 0.0, 0.0),
        jump_fuel: 0,
        is_jumping: false,
        id: id_allocator.allocate(),
        lateral_rotation: 0.0,
        vertical_rotation: 0.0,
        surroundings_loader:
          SurroundingsLoader::new(
            owner_allocator.allocate(),
            load_distance,
            Box::new(|&mut: d| LOD::LodIndex(Player::lod_index(d))),
          ),
        solid_boundary:
          SurroundingsLoader::new(
            owner_allocator.allocate(),
            1,
            Box::new(|&mut: _| LOD::Placeholder),
          ),
      };

      let min = Pnt3::new(0.0, terrain::AMPLITUDE as f32 * 0.6, 4.0);
      let max = min + Vec3::new(1.0, 2.0, 1.0);
      let bounds = AABB::new(min, max);
      physics.insert_misc(player.id, bounds.clone());

      // initialize the projection matrix
      player.camera.translate(center(&bounds).to_vec());
      player.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 2048.0);
      player.rotate_lateral(PI / 2.0);

      player
    };

    let misc_texture_unit = texture_unit_alloc.allocate();
    unsafe {
      gl::ActiveTexture(misc_texture_unit.gl_id());
    }

    let texture_in = hud_texture_shader.shader.get_uniform_location("texture_in");
    hud_texture_shader.shader.use_shader(gl_context);
    unsafe {
      gl::Uniform1i(texture_in, misc_texture_unit.glsl_id as GLint);
    }

    match gl_context.get_error() {
      gl::NO_ERROR => {},
      err => warn!("OpenGL error 0x{:x} in load()", err),
    }

    App {
      line_of_sight: line_of_sight,
      physics: physics,
      id_allocator: id_allocator,
      terrain_game_loader: terrain_game_loader,
      mob_buffers: mob_buffers,
      player: player,
      mobs: mobs,
      sun: 0,
      sun_timer: IntervalTimer::new(SUN_TICK_NS, time::precise_time_ns()),
      hud_triangles: hud_triangles,
      text_textures: text_textures,
      text_triangles: text_triangles,
      misc_texture_unit: misc_texture_unit,
      mob_shader: mob_shader,
      terrain_shader: terrain_shader,
      hud_color_shader: hud_color_shader,
      hud_texture_shader: hud_texture_shader,
      render_outlines: false,
      timers: timers,
      gl: gl,
      gl_context: gl_context,
    }
  }

  #[inline]
  fn get_bounds(&self, id: EntityId) -> &AABB3<f32> {
    self.physics.get_bounds(id).unwrap()
  }
}

fn add_mob(
  gl: &mut GLContext,
  physics: &mut Physics,
  mobs: &mut HashMap<EntityId, Rc<RefCell<mob::Mob>>>,
  mob_buffers: &mut mob::MobBuffers,
  id_allocator: &mut IdAllocator<EntityId>,
  owner_allocator: &mut IdAllocator<OwnerId>,
  low_corner: Pnt3<GLfloat>,
  behavior: mob::Behavior,
) {
  // TODO: mob loader instead of pushing directly to gl buffers

  let id = id_allocator.allocate();
  let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as GLfloat));

  let mob =
    mob::Mob {
      position: (*bounds.mins() + bounds.maxs().to_vec()) / 2.0,
      speed: Vec3::new(0.0, 0.0, 0.0),
      behavior: behavior,
      id: id,
      solid_boundary:
        SurroundingsLoader::new(owner_allocator.allocate(), 1, Box::new(|&: _| LOD::Placeholder)),
    };
  let mob = Rc::new(RefCell::new(mob));

  mob_buffers.push(gl, id, &to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0)));

  physics.insert_misc(id, bounds);
  mobs.insert(id, mob);
}

fn make_mobs<'a>(
  gl: &'a GLContextExistence,
  gl_context: & mut GLContext,
  physics: &mut Physics,
  id_allocator: &mut IdAllocator<EntityId>,
  owner_allocator: &mut IdAllocator<OwnerId>,
  shader: &shaders::color::ColorShader<'a>,
) -> (HashMap<EntityId, Rc<RefCell<mob::Mob<'a>>>>, mob::MobBuffers<'a>) {
  let mut mobs = HashMap::new();
  let mut mob_buffers = mob::MobBuffers::new(gl, gl_context, shader);

  fn mob_behavior(world: &App, mob: &mut mob::Mob) {
    let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
    if nalgebra::norm(&to_player) < 2.0 {
      mob.behavior = wait_for_distance;
    }

    fn wait_for_distance(world: &App, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if nalgebra::norm(&to_player) > 8.0 {
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
      if nalgebra::norm(&to_player) >= 2.0 {
        mob.behavior = mob_behavior;
      }
    }
  }

  add_mob(
    gl_context,
    physics,
    &mut mobs,
    &mut mob_buffers,
    id_allocator,
    owner_allocator,
    Pnt3::new(0.0, terrain::AMPLITUDE as f32 * 0.6, -1.0),
    mob_behavior
  );

  (mobs, mob_buffers)
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
