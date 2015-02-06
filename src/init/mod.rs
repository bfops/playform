pub mod hud;
pub mod mobs;
pub mod text;

use camera;
use color::Color4;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use init::hud::make_hud;
use init::mobs::make_mobs;
use init::text::make_text;
use lod_map::LOD;
use nalgebra::{Vec3, Pnt3};
use ncollide::bounding_volume::{AABB, AABB3};
use opencl_context::CL;
use physics::Physics;
use player::Player;
use shaders;
use state::App;
use stopwatch::TimerSet;
use std::f32::consts::PI;
use sun::Sun;
use surroundings_loader::SurroundingsLoader;
use terrain::terrain;
use terrain::terrain_game_loader::TerrainGameLoader;
use terrain::terrain_vram_buffers;
use vertex::ColoredVertex;
use yaglw::vertex_buffer::*;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::texture::TextureUnit;

const SUN_TICK_NS: u64 = 1000000;

fn center(bounds: &AABB3<f32>) -> Pnt3<GLfloat> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as GLfloat)
}

pub fn init<'a>(
  gl: &'a GLContextExistence,
  gl_context: &mut GLContext,
  shaders: &mut shaders::Shaders<'a>,
  cl: &CL,
  timers: &'a TimerSet,
) -> App<'a> {
  unsafe {
    gl::FrontFace(gl::CCW);
    gl::CullFace(gl::BACK);
    gl::Enable(gl::CULL_FACE);
  }
  gl_context.enable_alpha_blending();
  gl_context.enable_smooth_lines();
  gl_context.enable_depth_buffer(1.0);

  let line_of_sight = {
    let buffer = GLBuffer::new(gl, gl_context, 2 * 2);
    let mut line_of_sight = {
      GLArray::new(
        gl,
        gl_context,
        &shaders.mob_shader.shader,
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

  let hud_triangles = make_hud(gl, gl_context, &shaders.hud_color_shader.shader);

  let mut texture_unit_alloc: IdAllocator<TextureUnit> = IdAllocator::new();
  let terrain_game_loader =
    TerrainGameLoader::new(
      gl,
      gl_context,
      cl,
      &mut shaders.terrain_shader,
      &mut texture_unit_alloc,
    );

  let (text_textures, text_triangles) =
    make_text(gl, gl_context, &shaders.hud_texture_shader.shader);

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
        &shaders.mob_shader,
      )
    });

  let player = {
    let mut load_distance = Player::load_distance(terrain_vram_buffers::POLYGON_BUDGET as i32);

    // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
    let max_load_distance = 90;
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

    let min = Pnt3::new(0.0, terrain::AMPLITUDE as f32, 4.0);
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

  let texture_in = shaders.hud_texture_shader.shader.get_uniform_location("texture_in");
  shaders.hud_texture_shader.shader.use_shader(gl_context);
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
    sun: Sun::new(SUN_TICK_NS),
    hud_triangles: hud_triangles,
    text_textures: text_textures,
    text_triangles: text_triangles,
    misc_texture_unit: misc_texture_unit,
    render_outlines: false,
  }
}

