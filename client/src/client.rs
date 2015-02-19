//! Main Playform client state code.

use common::block_position::BlockPosition;
use common::lod::LODIndex;
use common::terrain_block::{TerrainBlock, LOD_QUALITY};
use nalgebra::Pnt3;
use std::collections::HashMap;
use std::iter::range_inclusive;
use std::sync::Mutex;
use terrain_buffers;

/// The distances at which LOD switches.
pub const LOD_THRESHOLDS: [i32; 3] = [1, 8, 32];

/// The main client state.
pub struct Client<'a> {
  #[allow(missing_docs)]
  pub player_position: Mutex<Pnt3<f32>>,
  #[allow(missing_docs)]
  pub max_load_distance: i32,
  /// A record of all the blocks that have been loaded.
  pub loaded_blocks: Mutex<HashMap<BlockPosition, (TerrainBlock, LODIndex)>>,
}

impl<'a> Client<'a> {
  #[allow(missing_docs)]
  pub fn new() -> Client<'a> {
    let mut load_distance = load_distance(terrain_buffers::POLYGON_BUDGET as i32);

    // TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
    let max_load_distance = 5;
    if load_distance > max_load_distance {
      info!("load_distance {} capped at {}", load_distance, max_load_distance);
      load_distance = max_load_distance;
    } else {
      info!("load_distance {}", load_distance);
    }

    Client {
      player_position: Mutex::new(Pnt3::new(0.0, 0.0, 0.0)),
      max_load_distance: load_distance,
      loaded_blocks: Mutex::new(HashMap::new()),
    }
  }
}

fn load_distance(mut polygon_budget: i32) -> i32 {
  // TODO: This should try to account for VRAM not used on a per-poly basis.

  let mut load_distance = 0;
  let mut prev_threshold = 0;
  let mut prev_square = 0;
  for (&threshold, &quality) in LOD_THRESHOLDS.iter().zip(LOD_QUALITY.iter()) {
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
    let quality = LOD_QUALITY[LOD_THRESHOLDS.len()];
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
