use cgmath::{Point, Vector, Point3, Vector3, EuclideanVector};

use voxel;

#[derive(Debug, Clone, Copy)]
pub struct T {
  pub center: Point3<f32>,
  pub radius: u8,
}

unsafe impl Send for T {}

impl voxel::field::T for T {
  fn density_at(this: &Self, p: &Point3<f32>) -> f32 {
    let d = this.center.sub_p(p);
    let r = this.radius as f32;
    r*r - (d.x*d.x + d.y*d.y + d.z*d.z)
  }

  fn contains(this: &Self, p: &Point3<f32>) -> bool {
    let d = this.center.sub_p(p);
    let r = this.radius as f32;
    d.x*d.x + d.y*d.y + d.z*d.z <= r*r
  }

  fn normal_at(this: &Self, _: f32, p: &Point3<f32>) -> Vector3<f32> {
    p.sub_p(&this.center).normalize()
  }
}

impl super::T for T {
  fn vertex_in(this: &Self, voxel: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)> {
    // The vertex will be placed on the surface of the sphere,
    // on the line formed by the center of the sphere and the center of the voxel.

    let d = voxel.center().sub_p(&this.center);
    let ratio = this.radius as f32 / d.length();
    let p = this.center.add_v(&d.mul_s(ratio));

    if voxel.contains(&p) {
      Some((
        voxel::Vertex::of_world_vertex_in(&p, voxel),
        voxel::Normal::of_float_normal(&d.normalize()),
      ))
    } else {
      None
    }
  }
}
