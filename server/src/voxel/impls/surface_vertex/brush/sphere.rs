use cgmath::{Point, Point3, Vector3, EuclideanVector};

mod voxel {
  pub use super::super::super::*;
}

#[derive(Debug, Clone, Copy)]
pub struct T {
  pub center: Point3<f32>,
  pub radius: f32,
}

unsafe impl Send for T {}

impl ::voxel::field::T for T {
  fn density_at(this: &Self, p: &Point3<f32>) -> f32 {
    let d = this.center.sub_p(p);
    this.radius*this.radius - (d.x*d.x + d.y*d.y + d.z*d.z)
  }

  fn contains(this: &Self, p: &Point3<f32>) -> bool {
    let d = this.center.sub_p(p);
    d.x*d.x + d.y*d.y + d.z*d.z <= this.radius*this.radius
  }

  fn normal_at(this: &Self, _: f32, p: &Point3<f32>) -> Vector3<f32> {
    p.sub_p(&this.center).normalize()
  }
}

impl voxel::brush::T for T {
  fn intersect(this: &Self, voxel: &::voxel::Bounds) -> voxel::brush::Intersection {
    match voxel::of_field(this, voxel) {
      voxel::T::Volume(true) => {
        voxel::brush::Intersection::Inside
      },
      voxel::T::Volume(false) => {
        voxel::brush::Intersection::Outside
      },
      voxel::T::Surface(surface) => {
        voxel::brush::Intersection::Crosses (
          surface.inner_vertex,
          surface.normal,
        )
      },
    }
  }
}
