//! Main Playform client state code.

use cgmath::Point3;
use num::iter::range_inclusive;
use std::sync::Mutex;

use common::entity_id;
use common::id_allocator;
use common::protocol;
use common::surroundings_loader::SurroundingsLoader;

use block_position;
use lod;
use terrain_mesh;
use terrain_buffers;

/// The distances at which LOD switches.
pub const LOD_THRESHOLDS: [i32; 3] = [2, 16, 32];

// TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
const MAX_LOAD_DISTANCE: i32 = 20;

/// The main client state.
pub struct T {
  #[allow(missing_docs)]
  pub id: protocol::ClientId,
  #[allow(missing_docs)]
  pub player_id: entity_id::T,
  #[allow(missing_docs)]
  pub player_position: Mutex<Point3<f32>>,
  #[allow(missing_docs)]
  pub max_load_distance: i32,
  #[allow(missing_docs)]
  pub surroundings_loader: Mutex<SurroundingsLoader>,
  pub id_allocator: Mutex<id_allocator::T<entity_id::T>>,
  /// A record of all the blocks that have been loaded.
  pub loaded_blocks: Mutex<block_position::Map<(terrain_mesh::T, lod::T)>>,
  pub partial_blocks: Mutex<block_position::Map<terrain_mesh::Partial>>,
  /// The number of terrain requests that are outstanding,
  pub outstanding_terrain_requests: Mutex<u32>,
}

#[allow(missing_docs)]
pub fn new(client_id: protocol::ClientId, player_id: entity_id::T, position: Point3<f32>) -> T {
  let mut load_distance = load_distance(terrain_buffers::POLYGON_BUDGET as i32);

  if load_distance > MAX_LOAD_DISTANCE {
    info!("load_distance {} capped at {}", load_distance, MAX_LOAD_DISTANCE);
    load_distance = MAX_LOAD_DISTANCE;
  } else {
    info!("load_distance {}", load_distance);
  }

  let surroundings_loader = {
    SurroundingsLoader::new(
      load_distance,
      LOD_THRESHOLDS.iter().cloned().collect(),
    )
  };

  T {
    id: client_id,
    player_id: player_id,
    player_position: Mutex::new(position),
    max_load_distance: load_distance,
    surroundings_loader: Mutex::new(surroundings_loader),
    id_allocator: Mutex::new(id_allocator::new()),
    partial_blocks: Mutex::new(block_position::new_map()),
    loaded_blocks: Mutex::new(block_position::new_map()),
    outstanding_terrain_requests: Mutex::new(0),
  }
}

unsafe impl Sync for T {}

fn load_distance(mut polygon_budget: i32) -> i32 {
  // TODO: This should try to account for VRAM not used on a per-poly basis.

  let mut load_distance = 0;
  let mut prev_threshold = 0;
  let mut prev_square = 0;
  for (&threshold, &quality) in LOD_THRESHOLDS.iter().zip(terrain_mesh::EDGE_SAMPLES.iter()) {
    let polygons_per_block = (quality * quality * 4) as i32;
    for i in range_inclusive(prev_threshold, threshold) {
      let i = 2 * i + 1;
      let square = i * i;
      let polygons_in_layer = (square - prev_square) * polygons_per_block;
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
    let quality = terrain_mesh::EDGE_SAMPLES[LOD_THRESHOLDS.len()];
    let polygons_per_block = (quality * quality * 4) as i32;
    let polygons_in_layer = (square - prev_square) * polygons_per_block;
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
