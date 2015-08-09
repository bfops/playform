use cgmath::Point3;

use super::super::field::Field;
use super::super::voxel;

/// Fields that can act as voxel brushes.
pub trait Brush: Field {
  /// Get a "representative" vertex for some voxel.
  fn vertex_in(&self, bounds: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)>;
}

/// The (approximate) area covered by the brush.
#[derive(Debug)]
pub struct Bounds {
  pub low: Point3<i32>,
  pub high: Point3<i32>,
}
