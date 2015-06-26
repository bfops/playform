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
  fn density_at(&self, x: f32, y: f32, z: f32) -> f32 {
    let coords = [x as f64, y as f64, z as f64];
    self.perlin.apply(&self.seed, &coords) as f32 - y/64.0
  }
}
