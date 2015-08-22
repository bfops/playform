use cgmath::Point3;
use noise::{Seed, Brownian2, Brownian3, perlin2, perlin3};

use voxel;

pub struct T {
  pub height: Brownian2<f64, fn (&Seed, &[f64; 2]) -> f64>,
  pub features: Brownian3<f64, fn (&Seed, &[f64; 3]) -> f64>,
  pub seed: Seed,
}

impl T {
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
      features:
        Brownian3::new(perlin3, 2)
        .frequency(1.0 / 32.0)
        .persistence(8.0)
        .lacunarity(1.0 / 4.0)
      ,
    }
  }
}

impl voxel::field::T for T {
  /// The height of the field at a given x,y,z.
  fn density_at(this: &Self, p: &Point3<f32>) -> f32 {
    let height = this.height.apply(&this.seed, &[p.x as f64, p.z as f64]);
    let height = height as f32;
    let heightmap_density = height - p.y;

    let feature_density = this.features.apply(&this.seed, &[p.x as f64, p.y as f64, p.z as f64]) * 8.0;
    let feature_density = feature_density as f32;

    heightmap_density + feature_density
  }
}
