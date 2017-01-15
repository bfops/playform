//! Structs for keeping track of terrain level of detail.

use num;

use chunk;

// TODO: terrain_mesh is now chunk-agnostic. Some/all of these values should be moved.
/// Number of LODs
pub const COUNT: usize = 5;

pub const ALL: [T; COUNT] = [T(0), T(1), T(2), T(3), T(4)];

/// lg(EDGE_SAMPLES)
// NOTE: If there are duplicates here, weird invariants will fail.
// Just remove the LODs if you don't want duplicates.
const LG_EDGE_SAMPLES: [u16; COUNT] = [3, 2, 1, 1, 0];

/// The width of a voxel within a chunk, indexed by LOD.
const LG_SAMPLE_SIZE: [i16; COUNT] = [
  chunk::LG_WIDTH as i16 - LG_EDGE_SAMPLES[0] as i16,
  chunk::LG_WIDTH as i16 - LG_EDGE_SAMPLES[1] as i16,
  chunk::LG_WIDTH as i16 - LG_EDGE_SAMPLES[2] as i16,
  chunk::LG_WIDTH as i16 - LG_EDGE_SAMPLES[3] as i16,
  chunk::LG_WIDTH as i16 - LG_EDGE_SAMPLES[4] as i16,
];

pub const MAX_GRASS_LOD: T = T(3);

/// The distances at which LOD switches.
pub const THRESHOLDS: [u32; COUNT-1] = [2, 16, 32, 48];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A strongly-typed index into various LOD-indexed arrays.
/// 0 is the highest LOD.
/// Ordering is "backwards": x > y means that x is bigger (lower level of detail) than y.
pub struct T(pub u32);

impl T {
  pub fn lg_sample_size(self) -> i16 {
    LG_SAMPLE_SIZE[self.0 as usize]
  }

  pub fn lg_edge_samples(self) -> u16 {
    LG_EDGE_SAMPLES[self.0 as usize]
  }

  pub fn edge_samples(self) -> u16 {
    1 << self.lg_edge_samples()
  }
}

pub fn of_distance(distance: u32) -> T {
  let mut lod = 0;
  while
    lod < THRESHOLDS.len()
    && THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  T(num::traits::FromPrimitive::from_usize(lod).unwrap())
}
