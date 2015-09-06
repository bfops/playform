/// A tree is comprised of a cylindrical trunk, a spherical bunch of leaves, and a spherical
/// rounding to the bottom of the trunk.

use cgmath::{Point, Point3, Vector3};

use voxel::*;

mod pillar {
  use cgmath::{Point, Point3, Vector3, EuclideanVector};

  use voxel::field;

  pub struct T {
    pub x: f32,
    pub z: f32,
    pub radius: f32,
  }

  impl field::T for T {
    fn density(this: &T, p: &Point3<f32>) -> f32 {
      let d = Point3::new(this.x, p.y, this.z).sub_p(p);
      this.radius*this.radius - d.length2()
    }

    fn normal(this: &T, p: &Point3<f32>) -> Vector3<f32> {
      Point3::new(this.x, p.y, this.z).sub_p(p).normalize()
    }
  }
}

pub struct T {
  union: mosaic::union::T,
}

unsafe impl Send for T {}

pub fn new(
  // Bottom-center of the trunk
  bottom: Point3<f32>, 
  trunk_height: f32, 
  trunk_radius: f32, 
  leaf_radius: f32,
) -> T {
  let leaf_center = bottom.add_v(&Vector3::new(0.0, trunk_height, 0.0));
  let trunk_center = bottom.add_v(&Vector3::new(0.0, trunk_height / 2.0, 0.0));

  let leaves =
    field::sphere::T {
      center: leaf_center,
      radius: leaf_radius,
    };

  let trunk = 
    field::intersection::new(
      pillar::T {
        x: bottom.x,
        z: bottom.z,
        radius: trunk_radius,
      },
      field::sphere::T {
        center: trunk_center,
        radius: trunk_height / 2.0,
      },
    );

  let mut union = mosaic::union::new();
  mosaic::union::push(&mut union, Material::Leaves, leaves);
  mosaic::union::push(&mut union, Material::Bark, trunk);

  T {
    union: union,
  }
}

impl field::T for T {
  fn density(this: &Self, p: &Point3<f32>) -> f32 {
    field::T::density(&this.union, p)
  }

  fn normal(this: &Self, p: &Point3<f32>) -> Vector3<f32> {
    field::T::normal(&this.union, p)
  }
}

impl mosaic::T for T {
  fn material(this: &Self, p: &Point3<f32>) -> Option<::voxel::Material> {
    mosaic::T::material(&this.union, p)
  }
}
