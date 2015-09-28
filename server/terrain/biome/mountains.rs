//! Mountain biome

use cgmath::{Point3, Vector3, EuclideanVector};
use noise::{Seed, Brownian2, Brownian3, perlin2, perlin3};

use voxel;
use voxel_base;

#[allow(missing_docs)]
pub struct T {
  pub height: Brownian2<f64, fn (&Seed, &[f64; 2]) -> f64>,
  pub features: Brownian3<f64, fn (&Seed, &[f64; 3]) -> f64>,
  pub seed: Seed,
}

#[allow(missing_docs)]
pub fn new(seed: Seed) -> T {
  let perlin2: fn(&Seed, &[f64; 2]) -> f64 = perlin2;
  let perlin3: fn(&Seed, &[f64; 3]) -> f64 = perlin3;
  T {
    seed: seed,
    height:
      Brownian2::new(perlin2, 5)
      .frequency(1.0 / 4.0)
      .persistence(4.0)
      .lacunarity(1.0 / 4.0)
    ,
    features:
      Brownian3::new(perlin3, 2)
      .frequency(1.0 / 32.0)
      .persistence(8.0)
      .lacunarity(1.0 / 4.0)
    ,
  }
}

impl voxel_base::field::T for T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    let height = self.height.apply(&self.seed, &[p.x as f64, p.z as f64]);
    let height = height as f32;
    let heightmap_density = height - p.y;

    let feature_density = self.features.apply(&self.seed, &[p.x as f64, p.y as f64, p.z as f64]) * 8.0;
    let feature_density = feature_density as f32;
    let feature_density = feature_density * 2.0;

    heightmap_density + feature_density
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    // Use density differential in each dimension as an approximation of the normal.

    let delta = 0.01;

    macro_rules! differential(($d:ident) => {{
      let high: f32 = {
        let mut p = *p;
        p.$d += delta;
        voxel_base::field::T::density(self, &p)
      };
      let low: f32 = {
        let mut p = *p;
        p.$d -= delta;
        voxel_base::field::T::density(self, &p)
      };
      high - low
    }});

    let v = Vector3::new(differential!(x), differential!(y), differential!(z));
    // Negate because we're leaving the volume when density is decreasing.
    let v = -v;
    v.normalize()
  }
}

impl voxel_base::mosaic::T for T {
  type Material = voxel::Material;

  fn material(&self, p: &Point3<f32>) -> Option<voxel::Material> {
    Some(
      if voxel_base::field::T::density(self, p) >= 0.0 {
        voxel::Material::Stone
      } else {
        voxel::Material::Empty
      }
    )
  }
}
