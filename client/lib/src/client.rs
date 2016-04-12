//! Main Playform client state code.

use cgmath::Point3;
use num::iter::range_inclusive;
use rand;
use rand::{Rng, SeedableRng};
use std::sync::Mutex;

use common::entity_id;
use common::id_allocator;
use common::protocol;
use common::surroundings_loader::SurroundingsLoader;
use common::voxel;

use loaded_edges;
use terrain_mesh;
use terrain_buffers;

/// The distances at which LOD switches.
pub const LOD_THRESHOLDS: [i32; terrain_mesh::LOD_COUNT-1] = [2, 16, 32];

// TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
const MAX_LOAD_DISTANCE: i32 = 1 << 6;

/// The main client state.
pub struct T {
  pub id: protocol::ClientId,
  pub player_id: entity_id::T,
  pub player_position: Mutex<Point3<f32>>,
  pub last_footstep: Mutex<Point3<f32>>,
  pub load_position: Mutex<Option<Point3<f32>>>,
  pub max_load_distance: i32,
  pub surroundings_loader: Mutex<SurroundingsLoader>,
  pub id_allocator: Mutex<id_allocator::T<entity_id::T>>,
  /// The set of currently loaded edges.
  pub loaded_edges: Mutex<loaded_edges::T<terrain_mesh::T>>,
  /// The voxels we have cached from the server.
  // TODO: Should probably remove from this at some point.
  pub voxels: Mutex<voxel::storage::T<voxel::T>>,
  /// The number of terrain requests that are outstanding,
  pub outstanding_terrain_requests: Mutex<u32>,
  pub rng: Mutex<rand::XorShiftRng>,
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

  let mut rng: rand::XorShiftRng = rand::SeedableRng::from_seed([1, 2, 3, 4]);
  let s1 = rng.next_u32();
  let s2 = rng.next_u32();
  let s3 = rng.next_u32();
  let s4 = rng.next_u32();
  rng.reseed([s1, s2, s3, s4]);

  T {
    id: client_id,
    player_id: player_id,
    player_position: Mutex::new(position),
    last_footstep: Mutex::new(position),
    load_position: Mutex::new(None),
    max_load_distance: load_distance,
    surroundings_loader: Mutex::new(surroundings_loader),
    id_allocator: Mutex::new(id_allocator::new()),
    loaded_edges: Mutex::new(loaded_edges::new()),
    voxels: Mutex::new(voxel::storage::new()),
    outstanding_terrain_requests: Mutex::new(0),
    rng: Mutex::new(rng),
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
