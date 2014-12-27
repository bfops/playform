use camera;
use color::Color4;
use common::*;
use fontloader;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use light::{Light, set_point_light, set_ambient_light};
use mob;
use nalgebra::{Pnt2, Vec2, Vec3, Pnt3, Norm};
use nalgebra;
use ncollide::bounding_volume::{AABB, AABB3};
use physics::Physics;
use player::Player;
use sdl2::mouse;
use shader;
use stopwatch::TimerSet;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::f32::consts::PI;
use std::rc::Rc;
use surroundings_loader::SurroundingsLoader;
use terrain_vram_buffers::TerrainVRAMBuffers;
use vertex::{ColoredVertex, TextureVertex};
use yaglw::vertex_buffer::*;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::{Texture2D, TextureUnit};

static SKY_COLOR: Color4<GLfloat>  = Color4 {r: 0.2, g: 0.5, b: 0.7, a: 1.0 };

#[deriving(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
pub struct EntityId(u32);

impl Default for EntityId {
  fn default() -> EntityId {
    EntityId(0)
  }
}

impl Add<u32, EntityId> for EntityId {
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
  shader: Rc<RefCell<Shader<'a>>>,
) -> (Vec<Texture2D<'a>>, GLArray<'a, TextureVertex>) {
  let fontloader = fontloader::FontLoader::new();
  let mut textures = Vec::new();
  let buffer = GLBuffer::new(gl, gl_context, 8 * VERTICES_PER_TRIANGLE);
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
  shader: Rc<RefCell<Shader<'a>>>,
) -> GLArray<'a ,ColoredVertex> {
  let buffer = GLBuffer::new(gl, gl_context, 16 * VERTICES_PER_TRIANGLE);
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
  pub player: Player,
  pub mobs: HashMap<EntityId, Rc<RefCell<mob::Mob>>>,

  pub id_allocator: IdAllocator<EntityId>,
  pub surroundings_loader: SurroundingsLoader,

  // OpenGL buffers
  pub terrain_buffers: TerrainVRAMBuffers<'a>,
  pub mob_buffers: mob::MobBuffers<'a>,
  pub line_of_sight: GLArray<'a, ColoredVertex>,
  pub hud_triangles: GLArray<'a, ColoredVertex>,
  pub text_triangles: GLArray<'a, TextureVertex>,

  pub misc_texture_unit: TextureUnit,
  pub text_textures: Vec<Texture2D<'a>>,

  // OpenGL shader "program" ids
  pub color_shader: Rc<RefCell<Shader<'a>>>,
  pub texture_shader: Rc<RefCell<Shader<'a>>>,
  pub hud_texture_shader: Rc<RefCell<Shader<'a>>>,
  pub hud_color_shader: Rc<RefCell<Shader<'a>>>,

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
    gl_context.enable_depth_buffer(100.0);
    gl_context.set_background_color(SKY_COLOR.r, SKY_COLOR.g, SKY_COLOR.b, SKY_COLOR.a);
    mouse::show_cursor(false);

    let texture_shader = {
      let texture_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          gl,
          String::from_str("shaders/world_texture"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &FromIterator::from_iter(
            [(String::from_str("lighting"), (true as uint).to_string())].to_vec().into_iter(),
          ),
        )));
      set_point_light(
        texture_shader.borrow_mut().deref_mut(),
        gl_context,
        &Light {
          position: Vec3::new(0.0, 16.0, 0.0),
          intensity: Vec3::new(0.6, 0.6, 0.6),
        }
      );
      set_ambient_light(
        texture_shader.borrow_mut().deref_mut(),
        gl_context,
        Vec3::new(0.4, 0.4, 0.4),
      );
      texture_shader
    };
    let color_shader =
      Rc::new(RefCell::new(shader::from_file_prefix(
        gl,
        String::from_str("shaders/color"),
        [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
        &HashMap::new(),
      )));
    let hud_color_shader =
      Rc::new(RefCell::new(shader::from_file_prefix(
        gl,
        String::from_str("shaders/color"),
        [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
        &HashMap::new(),
      )));
    let hud_texture_shader =
      Rc::new(RefCell::new(shader::from_file_prefix(
        gl,
        String::from_str("shaders/hud_texture"),
        [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
        &HashMap::new(),
      )));

    {
      let hud_camera = {
        let mut c = camera::Camera::unit();
        c.fov = camera::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
        c.fov = camera::translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
        c
      };

      camera::set_camera(
        hud_color_shader.borrow_mut().deref_mut(),
        gl_context,
        &hud_camera,
      );
      camera::set_camera(
        hud_texture_shader.borrow_mut().deref_mut(),
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
          color_shader.clone(),
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

    let hud_triangles = make_hud(gl, gl_context, hud_color_shader.clone());

    let mut texture_unit_alloc: IdAllocator<TextureUnit> = IdAllocator::new();
    let terrain_buffers = {
      let terrain_buffers = TerrainVRAMBuffers::new(gl, gl_context);
      terrain_buffers.bind_glsl_uniforms(gl_context, &mut texture_unit_alloc, texture_shader.clone());
      terrain_buffers
    };

    let (text_textures, text_triangles) =
      make_text(gl, gl_context, hud_texture_shader.clone());

    let mut physics =
      Physics::new(
        AABB::new(
          Pnt3 { x: -512.0, y: -32.0, z: -512.0 },
          Pnt3 { x: 512.0, y: 512.0, z: 512.0 },
        )
      );

    let mut id_allocator = IdAllocator::new();

    let (mobs, mob_buffers) =
      timers.time("make_mobs", || {
        make_mobs(
          gl,
          gl_context,
          &mut physics,
          &mut id_allocator,
          color_shader.clone(),
        )
      });

    let player = {
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
      };

      let min = Pnt3::new(0.0, 64.0, 4.0);
      let max = min + Vec3::new(1.0, 2.0, 1.0);
      let bounds = AABB::new(min, max);
      physics.insert_misc(player.id, &bounds);

      // initialize the projection matrix
      player.camera.translate(center(&bounds).to_vec());
      player.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);
      player.rotate_lateral(PI / 2.0);

      player
    };

    let misc_texture_unit = texture_unit_alloc.allocate();
    unsafe {
      gl::ActiveTexture(misc_texture_unit.gl_id());
    }

    let texture_in = hud_texture_shader.borrow_mut().get_uniform_location("texture_in");
    hud_texture_shader.borrow_mut().use_shader(gl_context);
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
      surroundings_loader: SurroundingsLoader::new(1),
      mob_buffers: mob_buffers,
      terrain_buffers: terrain_buffers,
      player: player,
      mobs: mobs,
      hud_triangles: hud_triangles,
      text_textures: text_textures,
      text_triangles: text_triangles,
      misc_texture_unit: misc_texture_unit,
      color_shader: color_shader,
      texture_shader: texture_shader,
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
    };
  let mob = Rc::new(RefCell::new(mob));

  mob_buffers.push(gl, id, &to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0)));

  physics.insert_misc(id, &bounds);
  mobs.insert(id, mob);
}

fn make_mobs<'a>(
  gl: &'a GLContextExistence,
  gl_context: & mut GLContext,
  physics: &mut Physics,
  id_allocator: &mut IdAllocator<EntityId>,
  shader: Rc<RefCell<Shader>>,
) -> (HashMap<EntityId, Rc<RefCell<mob::Mob>>>, mob::MobBuffers<'a>) {
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
    Pnt3::new(0.0, 64.0, -1.0),
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
