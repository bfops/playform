use cgmath::{Aabb3};

use voxel;

pub type Bounds = Aabb3<i32>;

/// The interface provided by voxel brushes.
pub trait T {
  /// The type of voxel this brush operates on.
  type Voxel;

  /// Use this brush to remove volume from a voxel.
  fn remove(
    this: &mut Self::Voxel,
    bounds: &voxel::Bounds,
    brush: &Self,
  );
}
