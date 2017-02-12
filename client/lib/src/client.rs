//! Main Playform client state code.

use cgmath::Point3;
use num;
use rand;
use rand::{Rng, SeedableRng};
use std::sync::Mutex;

use common::id_allocator;
use common::protocol;
use common::surroundings_loader;

use lod;
use terrain;
use view;

// TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
const MAX_LOAD_DISTANCE: u32 = 80;

/// The main client state.
pub struct T {
  #[allow(missing_docs)]
  pub id                       : protocol::ClientId,
  /// id for the player in vram
  pub player_id                : view::entity::id::Player,
  /// position of the player in world coordinates
  pub player_position          : Mutex<Point3<f32>>,
  /// the location where we last played a footstep sound
  pub last_footstep            : Mutex<Point3<f32>>,
  /// world position to center terrain loading around
  pub load_position            : Mutex<Option<Point3<f32>>>,
  #[allow(missing_docs)]
  pub terrain_allocator        : Mutex<id_allocator::T<view::entity::id::Terrain>>,
  #[allow(missing_docs)]
  pub grass_allocator          : Mutex<id_allocator::T<view::entity::id::Grass>>,
  #[allow(missing_docs)]
  pub surroundings_loader      : Mutex<surroundings_loader::T>,
  #[allow(missing_docs)]
  pub max_load_distance        : u32,
  #[allow(missing_docs)]
  pub terrain                  : Mutex<terrain::T>,
  /// The number of terrain requests that are outstanding,
  pub pending_terrain_requests : Mutex<u32>,
  #[allow(missing_docs)]
  pub rng                      : Mutex<rand::XorShiftRng>,
}

fn load_distance(mut polygon_budget: i32) -> u32 {
  // TODO: This should try to account for VRAM not used on a per-poly basis.

  let mut load_distance = 0;
  let mut prev_threshold = 0;
  let mut prev_square = 0;
  for (i, &threshold) in lod::THRESHOLDS.iter().enumerate() {
    let quality = lod::T(i as u32).edge_samples() as i32;
    let polygons_per_chunk = quality * quality * 4;
    for i in num::iter::range_inclusive(prev_threshold, threshold) {
      let i = 2 * i + 1;
      let square = i * i;
      let polygons_in_layer = (square - prev_square) as i32 * polygons_per_chunk;
      polygon_budget -= polygons_in_layer;
      if polygon_budget < 0 {
        break;
      }

      load_distance += 1;
      prev_square = square;
    }
    prev_threshold = threshold + 1;
  }

  let mut width = 2 * prev_threshold + 1;
  loop {
    let square = width * width;
    // The "to infinity and beyond" quality.
    let quality = lod::ALL.iter().last().unwrap().edge_samples() as i32;
    let polygons_per_chunk = quality * quality * 4;
    let polygons_in_layer = (square - prev_square) as i32 * polygons_per_chunk;
    polygon_budget -= polygons_in_layer;

    if polygon_budget < 0 {
      break;
    }

    width += 2;
    load_distance += 1;
    prev_square = square;
  }

  load_distance
}

#[allow(missing_docs)]
pub fn new(client_id: protocol::ClientId, player_id: view::entity::id::Player, position: Point3<f32>) -> T {
  let mut rng: rand::XorShiftRng = rand::SeedableRng::from_seed([1, 2, 3, 4]);
  let s1 = rng.next_u32();
  let s2 = rng.next_u32();
  let s3 = rng.next_u32();
  let s4 = rng.next_u32();
  rng.reseed([s1, s2, s3, s4]);

  let mut load_distance = load_distance(view::terrain_buffers::POLYGON_BUDGET as i32);

  if load_distance > MAX_LOAD_DISTANCE {
    info!("load_distance {} capped at {}", load_distance, MAX_LOAD_DISTANCE);
    load_distance = MAX_LOAD_DISTANCE;
  } else {
    info!("load_distance {}", load_distance);
  }

  let surroundings_loader = {
    surroundings_loader::new(
      load_distance,
      lod::THRESHOLDS.iter().map(|&x| x as i32).collect(),
    )
  };

  T {
    id                       : client_id,
    player_id                : player_id,
    player_position          : Mutex::new(position),
    last_footstep            : Mutex::new(position),
    load_position            : Mutex::new(None),
    terrain_allocator        : Mutex::new(id_allocator::new()),
    grass_allocator          : Mutex::new(id_allocator::new()),
    surroundings_loader      : Mutex::new(surroundings_loader),
    max_load_distance        : load_distance,
    terrain                  : Mutex::new(terrain::new(load_distance as u32)),
    pending_terrain_requests : Mutex::new(0),
    rng                      : Mutex::new(rng),
  }
}

unsafe impl Sync for T {}
