use cgmath::Point3;
use noise::{Seed, Brownian2, Brownian3, perlin2, perlin3};

use field::Field;

pub struct HeightMap {
  pub height: Brownian2<f64, fn (&Seed, &[f64; 2]) -> f64>,
  pub features: Brownian3<f64, fn (&Seed, &[f64; 3]) -> f64>,
  pub seed: Seed,
}

impl HeightMap {
  pub fn new(seed: Seed) -> HeightMap {
    let perlin2: fn(&Seed, &[f64; 2]) -> f64 = perlin2;
    let perlin3: fn(&Seed, &[f64; 3]) -> f64 = perlin3;
    HeightMap {
      seed: seed,
      height:
        Brownian2::new(perlin2, 4)
        .frequency(1.0 / 8.0)
        .persistence(8.0)
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
}

impl Field for HeightMap {
  /// The height of the field at a given x,y,z.
  fn density_at(&self, p: &Point3<f32>) -> f32 {
    let height = self.height.apply(&self.seed, &[p.x as f64, p.z as f64]);
    let height = height as f32;
    let heightmap_density = height - p.y;

    let feature_density = self.features.apply(&self.seed, &[p.x as f64, p.y as f64, p.z as f64]) * 8.0;
    let feature_density = feature_density as f32;

    heightmap_density + feature_density
  }
}
