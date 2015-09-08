/// A tree is comprised of a cylindrical trunk, a spherical bunch of leaves, and a spherical
/// rounding to the bottom of the trunk.

use cgmath::{Point, Point3, Vector3};

use voxel;
use voxel::{field, mosaic};

mod pillar {
  use cgmath::{Point, Point3, Vector3, EuclideanVector};

  use voxel::field;

  pub struct T {
    pub radius: f32,
  }

  unsafe impl Send for T {}

  impl field::T for T {
    fn density(&self, p: &Point3<f32>) -> f32 {
      let mut p = p.clone();
      p.y = 0.0;
      self.radius*self.radius - p.to_vec().length2()
    }

    fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
      let mut p = p.clone();
      p.y = 0.0;
      p.to_vec().normalize()
    }
  }
}

pub struct T {
  union: mosaic::union::T,
}

unsafe impl Send for T {}

pub fn new(
  trunk_height: f32,
  trunk_radius: f32,
  leaf_radius: f32,
) -> T {
  let leaf_center = Point3::new(0.0, trunk_height, 0.0);
  let trunk_center = Point3::new(0.0, trunk_height / 2.0, 0.0);

  let leaves =
    field::translation::T {
      translation: leaf_center.to_vec(),
      field: field::sphere::T {
        radius: leaf_radius,
      },
    };

  let trunk =
    field::translation::T {
      translation: trunk_center.to_vec(),
      field: field::intersection::new(
        pillar::T {
          radius: trunk_radius,
        },
        field::sphere::T {
          radius: trunk_height / 2.0,
        },
      ),
    };

  let mut union = mosaic::union::new();
  union.push(voxel::Material::Leaves, leaves);
  union.push(voxel::Material::Bark, trunk);

  T {
    union: union,
  }
}

impl field::T for T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    field::T::density(&self.union, p)
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    field::T::normal(&self.union, p)
  }
}

impl mosaic::T for T {
  fn material(&self, p: &Point3<f32>) -> Option<voxel::Material> {
    mosaic::T::material(&self.union, p)
  }
}
