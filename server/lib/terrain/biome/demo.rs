//! Grass, hilly biome

use cgmath::{Point3, Vector3, EuclideanVector};
use noise::{Seed, Brownian2, Brownian3, perlin2, perlin3};

use common::voxel;

#[allow(missing_docs)]
pub struct T {
  height: Brownian2<f64, fn (&Seed, &[f64; 2]) -> f64>,
  mountains: Brownian2<f64, fn (&Seed, &[f64; 2]) -> f64>,
  features: Brownian3<f64, fn (&Seed, &[f64; 3]) -> f64>,
  seed: Seed,
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
      .persistence(2.0)
      .lacunarity(1.0 / 2.0)
    ,
    mountains:
      Brownian2::new(perlin2, 3)
      .frequency(1.0 / 16.0)
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

impl T {
  fn mat_density(&self, p: &Point3<f32>) -> (f32, voxel::Material) {
    let height = self.height.apply(&self.seed, &[p.x as f64, p.z as f64]);
    let height = height as f32;
    let heightmap_density = height - p.y;

    let mountain_height = 16.0 * self.mountains.apply(&self.seed, &[p.x as f64 - 32.0, p.z as f64 - 10.0]) - 32.0;
    let mountain_height = mountain_height as f32;
    let mountain_heightmap_density = mountain_height - p.y;

    let feature_density = self.features.apply(&self.seed, &[p.x as f64, p.y as f64, p.z as f64]) * 8.0;
    let feature_density = feature_density as f32;
    let d = feature_density + heightmap_density;

    if mountain_heightmap_density > d {
      (mountain_heightmap_density, voxel::Material::Stone)
    } else {
      (d, voxel::Material::Terrain)
    }
  }
}

impl voxel::field::T for T {
  fn density(&mut self, p: &Point3<f32>) -> f32 {
    let (d, _) = self.mat_density(p);
    d
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
    let (d, mat) = self.mat_density(p);
    Some(
      if d >= 0.0 {
        mat
      } else {
        voxel::Material::Empty
      }
    )
  }
}
