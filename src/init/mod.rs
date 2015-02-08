pub mod hud;
pub mod mobs;
pub mod text;

use camera;
use gl;
use gl::types::*;
use id_allocator::IdAllocator;
use init::hud::make_hud;
use init::mobs::make_mobs;
use init::text::make_text;
use nalgebra::{Pnt3, Vec3};
use ncollide_entities::bounding_volume::{AABB, AABB3};
use opencl_context::CL;
use physics::Physics;
use player::Player;
use render_state::RenderState;
use world::World;
use std::f32::consts::PI;
use stopwatch::TimerSet;
use sun::Sun;
use terrain::terrain;
use terrain::terrain_game_loader::TerrainGameLoader;
use terrain::terrain_vram_buffers;

const SUN_TICK_NS: u64 = 5000000;

fn center(bounds: &AABB3<f32>) -> Pnt3<f32> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as f32)
}

pub fn init<'a, 'b:'a>(
  render_state: &mut RenderState<'b>,
  cl: &CL,
  timers: &TimerSet,
) -> World<'b> {
  unsafe {
    gl::FrontFace(gl::CCW);
    gl::CullFace(gl::BACK);
    gl::Enable(gl::CULL_FACE);
  }
  render_state.gl.enable_alpha_blending();
  render_state.gl.enable_smooth_lines();
  render_state.gl.enable_depth_buffer(1.0);

  make_hud(render_state);

  let terrain_game_loader = TerrainGameLoader::new(cl);

  make_text(render_state);

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
        render_state,
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

  let player = {
    let mut player = Player::new(
      &mut id_allocator,
      &mut owner_allocator,
      load_distance,
    );

    let min = Pnt3::new(0.0, terrain::AMPLITUDE as f32, 4.0);
    let max = min + Vec3::new(1.0, 2.0, 1.0);
    let bounds = AABB::new(min, max);
    physics.insert_misc(player.id, bounds.clone());

    let position = center(&bounds);
    player.position = position;

    // Initialize the projection matrix.
    render_state.camera.translate(position.to_vec());
    render_state.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 2048.0);

    player.rotate_lateral(PI / 2.0);
    render_state.rotate_lateral(PI / 2.0);

    player
  };

  unsafe {
    gl::ActiveTexture(render_state.misc_texture_unit.gl_id());
  }

  let texture_in =
    render_state.shaders.hud_texture_shader.shader.get_uniform_location("texture_in");
  render_state.shaders.hud_texture_shader.shader.use_shader(&mut render_state.gl);
  unsafe {
    gl::Uniform1i(texture_in, render_state.misc_texture_unit.glsl_id as GLint);
  }

  match render_state.gl.get_error() {
    gl::NO_ERROR => {},
    err => warn!("OpenGL error 0x{:x} in load()", err),
  }

  World {
    physics: physics,
    id_allocator: id_allocator,
    terrain_game_loader: terrain_game_loader,
    player: player,
    mobs: mobs,
    sun: Sun::new(SUN_TICK_NS),
  }
}

