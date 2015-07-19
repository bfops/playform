use cgmath::{Point, Vector, Point3, Vector3, EuclideanVector};

use super::field::Field;
use super::voxel;

pub trait Brush: Field {
  fn vertex_in(&self, bounds: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)>;
}

#[derive(Debug)]
pub struct Bounds {
  pub low: Point3<i32>,
  pub high: Point3<i32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Sphere {
  pub center: Point3<f32>,
  pub radius: u8,
}

unsafe impl Send for Sphere {}

impl Field for Sphere {
  fn density_at(&self, p: &Point3<f32>) -> f32 {
    let d = self.center.sub_p(p);
    let r = self.radius as f32;
    r*r - (d.x*d.x + d.y*d.y + d.z*d.z)
  }

  fn contains(&self, p: &Point3<f32>) -> bool {
    let d = self.center.sub_p(p);
    let r = self.radius as f32;
    d.x*d.x + d.y*d.y + d.z*d.z <= r*r
  }

  fn normal_at(&self, _: f32, p: &Point3<f32>) -> Vector3<f32> {
    p.sub_p(&self.center).normalize()
  }
}

impl Brush for Sphere {
  fn vertex_in(&self, voxel: &voxel::Bounds) -> Option<(voxel::Vertex, voxel::Normal)> {
    // The vertex will be placed on the surface of the sphere,
    // on the line formed by the center of the sphere and the center of the voxel.

    let d = voxel.center().sub_p(&self.center);
    let ratio = self.radius as f32 / d.length();
    let p = self.center.add_v(&d.mul_s(ratio));

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
