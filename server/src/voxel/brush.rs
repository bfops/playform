use cgmath::{Aabb3};

use voxel;

pub type Bounds = Aabb3<i32>;

pub trait T {
  type Voxel;

  fn remove(
    this: &mut Self::Voxel,
    bounds: &voxel::Bounds,
    brush: &Self,
    brush_bounds: &Bounds,
  );
}
