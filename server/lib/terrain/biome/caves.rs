//! Cave biome. This is very experimental and really needs occlusion culling to support any render distance at all.

use cgmath::{Point3, Vector3, EuclideanVector};
use noise::{Seed, perlin3};

use common::voxel;

#[allow(missing_docs)]
pub struct T {
  pub seed: Seed,
}

#[allow(missing_docs)]
pub fn new(seed: Seed) -> T {
  T {
    seed: seed,
  }
}

impl voxel::field::T for T {
  fn density(&mut self, p: &Point3<f32>) -> f32 {
    let freq = |f: f64| {
      perlin3(&self.seed, &[(p.x as f64) * f, (p.y as f64) * f, (p.z as f64) * f])
    };

    let d =
      freq(1.0 / 32.0) -
      f64::max(0.0, freq(1.0 / 16.0));
    d as f32
  }

  fn normal(&mut self, p: &Point3<f32>) -> Vector3<f32> {
    // Use density differential in each dimension as an approximation of the normal.

    let delta = 0.01;

    macro_rules! differential(($d:ident) => {{
      let high: f32 = {
        let mut p = *p;
        p.$d += delta;
        voxel::field::T::density(self, &p)
      };
      let low: f32 = {
        let mut p = *p;
        p.$d -= delta;
        voxel::field::T::density(self, &p)
      };
      high - low
    }});

    let v = Vector3::new(differential!(x), differential!(y), differential!(z));
    // Negate because we're leaving the volume when density is decreasing.
    let v = -v;
    v.normalize()
  }
}

impl voxel::mosaic::T<voxel::Material> for T {
  fn material(&mut self, p: &Point3<f32>) -> Option<voxel::Material> {
    Some(
      if voxel::field::T::density(self, p) >= 0.0 {
        voxel::Material::Stone
      } else {
        voxel::Material::Empty
      }
    )
  }
}
