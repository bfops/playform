use cgmath::{EuclideanVector, Vector3};
use noise::{Seed, Brownian3, perlin3};

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

  /// The height of the field at a given x,y,z.
  pub fn density_at(&self, x: f32, y: f32, z: f32) -> f32 {
    let coords = [x as f64, y as f64, z as f64];
    (self.perlin)(&self.seed, &coords) as f32 - y/64.0
  }

  /// The lighting normal of the tile at a given x,y,z.
  pub fn normal_at(&self, delta: f32, x: f32, y: f32, z: f32) -> Vector3<f32> {
    // Get the density differential in each dimension.
    // Use that as the approximate normal.

    let dx = self.density_at(x + delta, y, z) - self.density_at(x - delta, y, z);
    let dy = self.density_at(x, y + delta, z) - self.density_at(x, y - delta, z);
    let dz = self.density_at(x, y, z + delta) - self.density_at(x, y, z - delta);

    let v = Vector3::new(dx, dy, dz);

    // Negate because we're "leaving" the surface when density is *decreasing*.
    let v = -v.normalize();
    v
  }
}
