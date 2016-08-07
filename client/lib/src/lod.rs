//! Structs for keeping track of terrain level of detail.

use num;

use chunk;

// TODO: terrain_mesh is now chunk-agnostic. Some/all of these values should be moved.
/// Number of LODs
pub const COUNT: usize = 5;

/// lg(EDGE_SAMPLES)
// NOTE: If there are duplicates here, weird invariants will fail.
// Just remove the LODs if you don't want duplicates.
pub const LG_EDGE_SAMPLES: [u16; COUNT] = [3, 2, 1, 1, 0];
/// The number of voxels along an axis within a chunk, indexed by LOD.
pub const EDGE_SAMPLES: [u16; COUNT] = [
  1 << LG_EDGE_SAMPLES[0],
  1 << LG_EDGE_SAMPLES[1],
  1 << LG_EDGE_SAMPLES[2],
  1 << LG_EDGE_SAMPLES[3],
  1 << LG_EDGE_SAMPLES[4],
];

/// The width of a voxel within a chunk, indexed by LOD.
pub const LG_SAMPLE_SIZE: [i16; COUNT] = [
  chunk::LG_WIDTH - LG_EDGE_SAMPLES[0] as i16,
  chunk::LG_WIDTH - LG_EDGE_SAMPLES[1] as i16,
  chunk::LG_WIDTH - LG_EDGE_SAMPLES[2] as i16,
  chunk::LG_WIDTH - LG_EDGE_SAMPLES[3] as i16,
  chunk::LG_WIDTH - LG_EDGE_SAMPLES[4] as i16,
];

pub const MAX_GRASS_LOD: T = T(3);

/// The distances at which LOD switches.
pub const THRESHOLDS: [i32; COUNT-1] = [2, 16, 32, 48];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A strongly-typed index into various LOD-indexed arrays.
/// 0 is the highest LOD.
/// Ordering is "backwards": x > y means that x is bigger (lower level of detail) than y.
pub struct T(pub u32);

pub fn of_distance(distance: i32) -> T {
  assert!(distance >= 0);
  let mut lod = 0;
  while
    lod < THRESHOLDS.len()
    && THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  T(num::traits::FromPrimitive::from_usize(lod).unwrap())
}
