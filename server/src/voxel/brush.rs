/// Voxel brush trait. An invariant of these brushes is that they shouldn't ever end "perfectly" on
/// voxel boundaries, since an invariant of terrain generation is that any edges that cross the
/// surface must be adjacent to four neighbors that also cross the surface. This can be broken,
/// say, if you have a brush that ends exactly on the corner of a voxel, since in that case,
/// there's no vertex you can place inside the lower neighbor voxel (since the removed corner
/// belongs to only one voxel).

use cgmath::{Aabb3};

use voxel;

pub type Bounds = Aabb3<i32>;

#[derive(Debug, Copy, Clone)]
pub enum Action {
  Remove,
}

/// The interface provided by voxel brushes.
pub trait T {
  /// The type of voxel this brush operates on.
  type Voxel;

  /// Use this brush to remove volume from a voxel.
  fn apply(
    this: &mut Self::Voxel,
    bounds: &voxel::Bounds,
    brush: &Self,
    action: Action,
  );
}
