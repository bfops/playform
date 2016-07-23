//! Main Playform client state code.

use cgmath::Point3;
use num;
use rand;
use rand::{Rng, SeedableRng};
use std::sync::Mutex;

use common::chunk;
use common::entity_id;
use common::fnv_set;
use common::id_allocator;
use common::protocol;
use common::surroundings_loader;

<<<<<<< HEAD
use loaded_edges;
use terrain_buffers;
use terrain_loader;
use terrain_mesh;
=======
use lod;
use terrain_mesh;
use terrain;
use view;
>>>>>>> master

/// The distances at which LOD switches.
pub const LOD_THRESHOLDS: [i32; terrain_mesh::LOD_COUNT-1] = [2, 16, 32];
// TODO: Remove this once our RAM usage doesn't skyrocket with load distance.
const MAX_LOAD_DISTANCE: i32 = 1 << 6;

pub fn lod_index(distance: i32) -> lod::T {
  assert!(distance >= 0);
  let mut lod = 0;
  while
    lod < LOD_THRESHOLDS.len()
    && LOD_THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  lod::T(num::traits::FromPrimitive::from_usize(lod).unwrap())
}

/// The main client state.
pub struct T {
  pub id                       : protocol::ClientId,
  pub player_id                : entity_id::T,
  pub player_position          : Mutex<Point3<f32>>,
  pub last_footstep            : Mutex<Point3<f32>>,
  pub load_position            : Mutex<Option<Point3<f32>>>,
<<<<<<< HEAD
  pub max_load_distance        : i32,
  pub surroundings_loader      : Mutex<SurroundingsLoader>,
  pub id_allocator             : Mutex<id_allocator::T<entity_id::T>>,
  /// The set of currently loaded edges.
  pub loaded_edges             : Mutex<loaded_edges::T<terrain_mesh::T>>,
  /// The voxels we have cached from the server.
  pub voxels                   : Mutex<voxel::tree::T>,
  /// The terrain requests that are pending,
  pub pending_terrain_requests : Mutex<fnv_set::T<chunk::Position>>,
  pub terrain_loader           : Mutex<terrain_loader::T>,
  pub rng                      : Mutex<rand::XorShiftRng>,
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
    id                       : client_id,
    player_id                : player_id,
    player_position          : Mutex::new(position),
    last_footstep            : Mutex::new(position),
    load_position            : Mutex::new(None),
    max_load_distance        : load_distance,
    surroundings_loader      : Mutex::new(surroundings_loader),
    id_allocator             : Mutex::new(id_allocator::new()),
    loaded_edges             : Mutex::new(loaded_edges::new()),
    voxels                   : Mutex::new(voxel::tree::new()),
    pending_terrain_requests : Mutex::new(fnv_set::new()),
    terrain_loader           : Mutex::new(terrain_loader::new()),
    rng                      : Mutex::new(rng),
  }
=======
  pub id_allocator             : Mutex<id_allocator::T<entity_id::T>>,
  pub surroundings_loader      : Mutex<surroundings_loader::T>,
  pub max_load_distance        : i32,
  pub terrain                  : Mutex<terrain::T>,
  /// The number of terrain requests that are outstanding,
  pub pending_terrain_requests : Mutex<u32>,
  pub rng                      : Mutex<rand::XorShiftRng>,
>>>>>>> master
}

fn load_distance(mut polygon_budget: i32) -> i32 {
  // TODO: This should try to account for VRAM not used on a per-poly basis.

  let mut load_distance = 0;
  let mut prev_threshold = 0;
  let mut prev_square = 0;
  for (&threshold, &quality) in LOD_THRESHOLDS.iter().zip(terrain_mesh::EDGE_SAMPLES.iter()) {
    let polygons_per_chunk = (quality * quality * 4) as i32;
    for i in num::iter::range_inclusive(prev_threshold, threshold) {
      let i = 2 * i + 1;
      let square = i * i;
      let polygons_in_layer = (square - prev_square) * polygons_per_chunk;
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
    let polygons_per_chunk = (quality * quality * 4) as i32;
    let polygons_in_layer = (square - prev_square) * polygons_per_chunk;
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
pub fn new(client_id: protocol::ClientId, player_id: entity_id::T, position: Point3<f32>) -> T {
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
      LOD_THRESHOLDS.iter().cloned().collect(),
    )
  };

  T {
    id                       : client_id,
    player_id                : player_id,
    player_position          : Mutex::new(position),
    last_footstep            : Mutex::new(position),
    load_position            : Mutex::new(None),
    id_allocator             : Mutex::new(id_allocator::new()),
    surroundings_loader      : Mutex::new(surroundings_loader),
    max_load_distance        : load_distance,
    terrain                  : Mutex::new(terrain::new(load_distance)),
    pending_terrain_requests : Mutex::new(0),
    rng                      : Mutex::new(rng),
  }
}

unsafe impl Sync for T {}
