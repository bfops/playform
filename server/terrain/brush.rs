use cgmath::Point3;

use field;
use voxel;

pub trait Brush: field::Field {
  fn vertex_in(&self, bounds: &voxel::Bounds) -> (voxel::Vertex, voxel::Normal);
}

pub struct Bounds {
  pub low: Point3<i32>,
  pub high: Point3<i32>,
}
