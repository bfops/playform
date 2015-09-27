//! A tree is comprised of a cylindrical trunk, a spherical bunch of leaves, and a spherical
//! rounding to the bottom of the trunk.

use cgmath::{Point, Point3, Vector, Vector3, EuclideanVector, Rotation};
use rand;

use voxel;
use voxel_base;
use voxel_base::field;
use voxel_base::mosaic;

mod pillar {
  use cgmath::{Point, Point3, Vector3, EuclideanVector};

  use voxel_base::field;

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

#[allow(missing_docs)]
pub struct T {
  union: voxel_base::mosaic::union::T<voxel::Material>,
}

unsafe impl Send for T {}

fn inside_sphere<Rng>(
  rng: &mut Rng,
  radius: f32,
) -> Point3<f32>
  where Rng: rand::Rng,
{
  loop {
    let p =
      Point3::new(
        rng.gen_range(-radius, radius),
        rng.gen_range(-radius, radius),
        rng.gen_range(-radius, radius),
      );
    if p.to_vec().length2() < radius*radius {
      return p
    }
  }
}

#[allow(missing_docs)]
pub fn new<Rng>(
  rng: &mut Rng,
  trunk_height: f32,
  trunk_radius: f32,
  leaf_radius: f32,
) -> T
  where Rng: rand::Rng,
{
  let trunk_top = Point3::new(0.0, trunk_height, 0.0);
  let leaf_center = trunk_top.add_v(&Vector3::new(0.0, leaf_radius / 2.0, 0.0));
  let trunk_center = Point3::new(0.0, trunk_height / 2.0, 0.0);

  let mut union = mosaic::union::new();

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

  union.push(voxel::Material::Bark, trunk);

  let leaf_count = {
    let leaf_radius = leaf_radius / 3.0;
    (leaf_radius * leaf_radius * leaf_radius) as i32
  };
  for _ in 0..leaf_count {
    let branch_end = leaf_center.add_v(&inside_sphere(rng, leaf_radius).to_vec());
    let branch = branch_end.sub_p(&trunk_top);
    let half_length = branch.length() / 2.0;

    union.push(
      voxel::Material::Bark,
      field::translation::T {
        translation: trunk_top.to_vec(),
        field: field::rotation::T {
          rotation: Rotation::between_vectors(&Vector3::new(0.0, 1.0, 0.0), &branch.normalize()),
          field: field::translation::T {
            translation: Vector3::new(0.0, half_length, 0.0),
            field: field::intersection::new(
              field::sphere::T {
                radius: half_length,
              },
              pillar::T {
                radius: 0.5,
              },
            ),
          },
        },
      },
    );

    union.push(
      voxel::Material::Leaves,
      field::translation::T {
        translation: branch_end.to_vec(),
        field: field::sphere::T {
          radius: 4.0,
        },
      },
    );
  }

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
  type Material = voxel::Material;

  fn material(&self, p: &Point3<f32>) -> Option<voxel::Material> {
    mosaic::T::material(&self.union, p)
  }
}
