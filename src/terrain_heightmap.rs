use nalgebra::{normalize, cross};
use nalgebra::{Pnt3, Vec3};
use noise::{Seed, Brownian2, perlin2, Point2};

pub struct HeightMap {
  pub perlin: Brownian2<f64, fn (&Seed, &Point2<f64>) -> f64>,
  pub amplitude: f64,
  pub seed: Seed,
}

impl HeightMap {
  pub fn new(
    seed: Seed,
    octaves: usize,
    frequency: f64,
    persistence: f64,
    lacunarity: f64,
    amplitude: f64,
  ) -> HeightMap {
    let perlin2: fn(&Seed, &Point2<f64>) -> f64 = perlin2;
    HeightMap {
      seed: seed,
      amplitude: amplitude,
      perlin:
        Brownian2::new(perlin2, octaves)
        .frequency(frequency)
        .persistence(persistence)
        .lacunarity(lacunarity),
    }
  }

  /// The coordinate of the tile at a given x/z.
  pub fn height_at(&self, x: f32, z: f32) -> f32 {
    (self.amplitude * ((self.perlin)(&self.seed, &[x as f64, z as f64]) + 1.0) / 2.0) as f32
  }

  /// The coordinate of the tile at a given x/z.
  pub fn point_at(&self, x: f32, z: f32) -> Pnt3<f32> {
    let y = self.height_at(x, z);
    Pnt3::new(x, y, z)
  }

  /// The lighting normal of the tile at a given x/z.
  pub fn normal_at(&self, delta: f32, x: f32, z: f32) -> Vec3<f32> {
    let dx =
      self.point_at(x + delta, z).to_vec()
      - self.point_at(x - delta, z).to_vec();
    let dz =
      self.point_at(x, z + delta).to_vec()
      - self.point_at(x, z - delta).to_vec();
    normalize(&cross(&dz, &dx))
  }
}
