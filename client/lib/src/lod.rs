//! Structs for keeping track of terrain level of detail.

use cgmath;
use num;

use client;
use chunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A strongly-typed index into various LOD-indexed arrays.
/// 0 is the highest LOD.
/// Ordering is "backwards": x > y means that x is bigger (lower level of detail) than y.
pub struct T(pub u32);

pub fn of_distance(distance: i32) -> T {
  assert!(distance >= 0);
  let mut lod = 0;
  while
    lod < client::LOD_THRESHOLDS.len()
    && client::LOD_THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  T(num::traits::FromPrimitive::from_usize(lod).unwrap())
}

pub fn at_chunk(player_position: &cgmath::Point3<f32>, chunk: &chunk::position::T) -> T {

}
