use cgmath::Point3;
use noise::{Seed, Brownian3, perlin3};

use field::Field;

pub struct HeightMap {
  pub perlin: Brownian3<f64, fn (&Seed, &[f64; 3]) -> f64>,
  pub seed: Seed,
}

impl HeightMap {
  pub fn new(
    seed: Seed,
    octaves: usize,
    frequency: f64,
    persistence: f64,
    lacunarity: f64,
  ) -> HeightMap {
    let perlin3: fn(&Seed, &[f64; 3]) -> f64 = perlin3;
    HeightMap {
      seed: seed,
      perlin:
        Brownian3::new(perlin3, octaves)
        .frequency(frequency)
        .persistence(persistence)
        .lacunarity(lacunarity),
    }
  }
}

impl Field for HeightMap {
  /// The height of the field at a given x,y,z.
  fn density_at(&self, p: &Point3<f32>) -> f32 {
    let coords = [p.x as f64, p.y as f64, p.z as f64];
    self.perlin.apply(&self.seed, &coords) as f32 - p.y/64.0
  }
}
