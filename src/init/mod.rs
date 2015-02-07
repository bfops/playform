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
use ncollide_entities::bounding_volume::AABB;
use opencl_context::CL;
use physics::Physics;
use player::Player;
use renderer::Renderer;
use state::App;
use stopwatch::TimerSet;
use sun::Sun;
use terrain::terrain;
use terrain::terrain_game_loader::TerrainGameLoader;
use terrain::terrain_vram_buffers;

const SUN_TICK_NS: u64 = 5000000;

pub fn init<'a, 'b:'a>(
  renderer: &mut Renderer<'b>,
  cl: &CL,
  timers: &TimerSet,
) -> App<'b> {
  unsafe {
    gl::FrontFace(gl::CCW);
    gl::CullFace(gl::BACK);
    gl::Enable(gl::CULL_FACE);
  }
  renderer.gl.enable_alpha_blending();
  renderer.gl.enable_smooth_lines();
  renderer.gl.enable_depth_buffer(1.0);

  make_hud(renderer);

  let terrain_game_loader = TerrainGameLoader::new(cl);

  make_text(renderer);

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

  let mobs =
    timers.time("make_mobs", || {
      make_mobs(
        renderer,
        &mut physics,
        &mut id_allocator,
        &mut owner_allocator,
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

  unsafe {
    gl::ActiveTexture(renderer.misc_texture_unit.gl_id());
  }

  let texture_in =
    renderer.shaders.hud_texture_shader.shader.get_uniform_location("texture_in");
  renderer.shaders.hud_texture_shader.shader.use_shader(&mut renderer.gl);
  unsafe {
    gl::Uniform1i(texture_in, renderer.misc_texture_unit.glsl_id as GLint);
  }

  match renderer.gl.get_error() {
    gl::NO_ERROR => {},
    err => warn!("OpenGL error 0x{:x} in load()", err),
  }

  App {
    physics: physics,
    id_allocator: id_allocator,
    terrain_game_loader: terrain_game_loader,
    player: player,
    mobs: mobs,
    sun: Sun::new(SUN_TICK_NS),
    render_outlines: false,
  }
}

