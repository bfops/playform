/// A tree is comprised of a cylindrical trunk, a spherical bunch of leaves, and a spherical
/// rounding to the bottom of the trunk.

use cgmath::{Point, Point3, Vector3, EuclideanVector};
use std::f32;

mod voxel {
  pub use super::super::super::*;
}

// TODO: More obviously compose this tree out of simpler shapes.

#[derive(Debug, Clone, Copy)]
pub struct T {
  // bottom-center of the trunk
  pub bottom: Point3<f32>,
  pub trunk_radius: f32,
  pub trunk_height: f32,
  pub leaf_radius: f32,
}

unsafe impl Send for T {}

fn leaf_center(this: &T) -> Point3<f32> {
  this.bottom.add_v(&Vector3::new(0.0, this.trunk_height, 0.0))
}

fn trunk_density(this: &T, p: &Point3<f32>) -> f32 {
  let center = this.bottom.add_v(&Vector3::new(0.0, this.trunk_height / 2.0, 0.0));

  let dbottom = p.sub_p(&this.bottom);
  let dtrunk = p.sub_p(&center);
  let mut dxz = dtrunk;
  dxz.y = 0.0;

  f32::max(
    this.trunk_radius - dbottom.length(),
    f32::min(
      this.trunk_height / 2.0 - dtrunk.length(),
      this.trunk_radius - dxz.length(),
    ),
  )
}

fn leaf_density(this: &T, p: &Point3<f32>) -> f32 {
  let d = leaf_center(this).sub_p(p);
  this.leaf_radius*this.leaf_radius - (d.x*d.x + d.y*d.y + d.z*d.z)
}

impl ::voxel::field::T for T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    f32::max(trunk_density(this, p), leaf_density(this, p))
  }

  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32> {
    let trunk_density = trunk_density(this, p);
    let leaf_density = leaf_density(this, p);
    if trunk_density < leaf_density {
      let leaf_center = leaf_center(this);
      p.sub_p(&leaf_center).normalize()
    } else {
      let n = p.sub_p(&this.bottom);
      if p.y <= this.bottom.y {
        // inside the rounded bottom of the trunk
        n.normalize()
      } else {
        let mut n = n;
        // inside the main trunk
        n.y = 0.0;
        n.normalize()
      }
    }
  }
}

impl ::voxel::mosaic::T for T {
  fn material(this: &Self, p: &Point3<f32>) -> Option<::voxel::Material> {
    let trunk_density = trunk_density(this, p);
    let leaf_density = leaf_density(this, p);
    if trunk_density < 0.0 && leaf_density < 0.0 {
      None
    } else {
      Some(
        if trunk_density >= leaf_density {
          ::voxel::Material::Bark
        } else {
          ::voxel::Material::Leaves
        }
      )
    }
  }
}
