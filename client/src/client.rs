//! Main Playform client state code.

use cgmath::Point3;
use std::collections::HashMap;
use std::iter::range_inclusive;
use std::sync::Mutex;

use common::block_position::BlockPosition;
use common::communicate::ClientId;
use common::entity::EntityId;
use common::lod::LODIndex;
use common::surroundings_loader::SurroundingsLoader;
use common::terrain_block;
use common::terrain_block::TerrainBlock;

use terrain_buffers;

/// The distances at which LOD switches.
pub const LOD_THRESHOLDS: [i32; 3] = [2, 16, 32];

/// The main client state.
pub struct T {
  #[allow(missing_docs)]
  pub id: ClientId,
  #[allow(missing_docs)]
  pub player_id: EntityId,
  #[allow(missing_docs)]
  pub player_position: Mutex<Point3<f32>>,
  #[allow(missing_docs)]
  pub max_load_distance: i32,
  #[allow(missing_docs)]
  pub surroundings_loader: Mutex<SurroundingsLoader>,
  /// A record of all the blocks that have been loaded.
  pub loaded_blocks: Mutex<HashMap<BlockPosition, (TerrainBlock, LODIndex)>>,
}

#[allow(missing_docs)]
pub fn new(client_id: ClientId, player_id: EntityId, position: Point3<f32>) -> T {
  let mut load_distance = load_distance(terrain_buffers::POLYGON_BUDGET as i32);

  // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
  let max_load_distance = 80;
  if load_distance > max_load_distance {
    info!("load_distance {} capped at {}", load_distance, max_load_distance);
    load_distance = max_load_distance;
  } else {
    info!("load_distance {}", load_distance);
  }

  let surroundings_loader = {
    SurroundingsLoader::new(
      max_load_distance,
      LOD_THRESHOLDS.iter().map(|&x| x).collect(),
    )
  };

  T {
    id: client_id,
    player_id: player_id,
    player_position: Mutex::new(position),
    max_load_distance: load_distance,
    surroundings_loader: Mutex::new(surroundings_loader),
    loaded_blocks: Mutex::new(HashMap::new()),
  }
}

unsafe impl Sync for T {}

fn load_distance(mut polygon_budget: i32) -> i32 {
  // TODO: This should try to account for VRAM not used on a per-poly basis.

  let mut load_distance = 0;
  let mut prev_threshold = 0;
  let mut prev_square = 0;
  for (&threshold, &quality) in LOD_THRESHOLDS.iter().zip(terrain_block::EDGE_SAMPLES.iter()) {
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
    let quality = terrain_block::EDGE_SAMPLES[LOD_THRESHOLDS.len()];
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
