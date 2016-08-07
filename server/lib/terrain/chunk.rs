//! Server-specific extensions to the chunk module.

use common::voxel;

pub use common::chunk::*;

/// Construct a chunk from a position and an initialization callback.
pub fn of_callback<F>(position: &position::T, lg_voxel_size: i16, mut f: F) -> T
  where F: FnMut(voxel::bounds::T) -> voxel::T
{
  assert!(lg_voxel_size <= 0 || lg_voxel_size as u16 <= LG_WIDTH);

  let mut voxels = Vec::new();

  let samples = 1 << (LG_WIDTH as i32 - lg_voxel_size as i32);
  for x in 0 .. samples {
  for y in 0 .. samples {
  for z in 0 .. samples {
    let bounds =
      voxel::bounds::T {
        x: position.as_point.x + x,
        y: position.as_point.y + y,
        z: position.as_point.z + z,
        lg_size: lg_voxel_size,
      };
    voxels.push(f(bounds));
  }}}

  T {
    voxels : voxels,
    width  : samples as u32,
  }
}

