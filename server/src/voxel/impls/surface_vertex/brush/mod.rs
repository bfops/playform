use voxel;

pub mod cube;

pub enum Intersection {
  /// The voxel is entirely outside the brush.
  Outside,
  /// The voxel is entirely inside the brush.
  Inside,
  /// The voxel crosses the brush surface.
  Crosses (voxel::Vertex, voxel::Normal),
}

/// Fields that can act as voxel brushes.
pub trait T: voxel::field::T {
  /// Get a "representative" vertex for some voxel.
  fn intersect(&Self, bounds: &voxel::Bounds) -> Intersection;
}
