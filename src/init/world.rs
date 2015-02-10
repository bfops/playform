use client_update::ServerToClient;
use id_allocator::IdAllocator;
use init::mobs::make_mobs;
use nalgebra::{Pnt3, Vec3};
use ncollide_entities::bounding_volume::{AABB, AABB3};
use opencl_context::CL;
use physics::Physics;
use player::Player;
use server::Server;
use std::f32::consts::PI;
use std::sync::mpsc::Sender;
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
  cl: &CL,
  view: &Sender<ServerToClient>,
  timers: &TimerSet,
) -> Server<'b> {
  let terrain_game_loader = TerrainGameLoader::new(cl);

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
        view,
        &mut physics,
        &mut id_allocator,
        &mut owner_allocator,
      )
    });

  let mut load_distance =
    Player::load_distance(terrain_vram_buffers::POLYGON_BUDGET as i32);

  // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
  let max_load_distance = 80;
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

    player.position = center(&bounds);

    player.rotate_lateral(PI / 2.0);

    player
  };

  Server {
    physics: physics,
    id_allocator: id_allocator,
    terrain_game_loader: terrain_game_loader,
    player: player,
    mobs: mobs,
    sun: Sun::new(SUN_TICK_NS),
  }
}

