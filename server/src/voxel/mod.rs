pub mod brush;
pub mod field;
pub mod mosaic;
pub mod tree;

pub mod impls;

use cgmath::{Point, Point3, Vector, Vector3};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Material {
  Empty = 0,
  Terrain = 1,
  Bark = 2,
  Leaves = 3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Bounds {
  /// x-coordinate as a multiple of 2^lg_size.
  pub x: i32,
  /// y-coordinate as a multiple of 2^lg_size.
  pub y: i32,
  /// z-coordinate as a multiple of 2^lg_size.
  pub z: i32,
  /// The log_2 of the voxel's size.
  pub lg_size: i16,
}

impl Bounds {
  /// Convenience function to create `Bounds`.
  /// N.B. That the input coordinates should be divided by (2^lg_size) relative to world coords.
  pub fn new(x: i32, y: i32, z: i32, lg_size: i16) -> Bounds {
    let ret =
      Bounds {
        x: x,
        y: y,
        z: z,
        lg_size: lg_size,
      };
    ret
  }

  /// The width of this voxel.
  pub fn size(&self) -> f32 {
    if self.lg_size >= 0 {
      (1 << self.lg_size) as f32
    } else {
      1.0 / (1 << -self.lg_size) as f32
    }
  }

  #[allow(dead_code)]
  pub fn low_corner(&self) -> Point3<f32> {
    let size = self.size();
    Point3::new(self.x as f32, self.y as f32, self.z as f32).mul_s(size)
  }

  pub fn corners(&self) -> (Point3<f32>, Point3<f32>) {
    let size = self.size();
    let low = Point3::new(self.x as f32, self.y as f32, self.z as f32).mul_s(size);
    (low, low.add_v(&Vector3::new(size, size, size)))
  }

  pub fn center(&self) -> Point3<f32> {
    let size = self.size();
    let half = Vector3::new(0.5, 0.5, 0.5);
    Point3::new(self.x as f32, self.y as f32, self.z as f32).add_v(&half).mul_s(size)
  }

  #[allow(dead_code)]
  pub fn contains(&self, p: &Point3<f32>) -> bool {
    let (low, high) = self.corners();
    p.x >= low.x &&
    p.y >= low.y &&
    p.z >= low.z &&
    p.x < high.x &&
    p.y < high.y &&
    p.z < high.z &&
    true
  }
}

/// The interface provided by Voxels.
pub trait T {
  /// Apply a brush to this voxel.
  fn brush<Mosaic>(
    this: &mut Self,
    bounds: &Bounds,
    brush: &brush::T<Mosaic>,
  ) where Mosaic: ::voxel::mosaic::T;
}
