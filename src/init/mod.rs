pub mod hud;
pub mod mobs;
pub mod text;

use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use init::hud::make_hud;
use init::mobs::make_mobs;
use init::text::make_text;
use nalgebra::Pnt3;
use ncollide::bounding_volume::AABB;
use opencl_context::CL;
use physics::Physics;
use player::Player;
use shaders;
use state::App;
use stopwatch::TimerSet;
use sun::Sun;
use terrain::terrain;
use terrain::terrain_game_loader::TerrainGameLoader;
use terrain::terrain_vram_buffers;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::texture::TextureUnit;

const SUN_TICK_NS: u64 = 5000000;

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

  let mut load_distance =
    Player::load_distance(terrain_vram_buffers::POLYGON_BUDGET as i32);

  // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
  let max_load_distance = 90;
  if load_distance > max_load_distance {
    info!("load_distance {} capped at {}", load_distance, max_load_distance);
    load_distance = max_load_distance;
  } else {
    info!("load_distance {}", load_distance);
  }

  let player =
    Player::new(
      &mut id_allocator,
      &mut owner_allocator,
      &mut physics,
      load_distance,
    );

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

