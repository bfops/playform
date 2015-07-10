use cgmath::{EuclideanVector, Vector3};

pub trait Field {
  /// The height of the field at a given x,y,z.
  fn density_at(&self, x: f32, y: f32, z: f32) -> f32;

  fn contains(&self, x: f32, y: f32, z: f32) -> bool {
    self.density_at(x, y, z) >= 0.0
  }

  /// The lighting normal of the tile at a given x,y,z.
  fn normal_at(&self, delta: f32, x: f32, y: f32, z: f32) -> Vector3<f32> {
    // Get the density differential in each dimension.
    // Use that as the approximate normal.

    let dx = self.density_at(x + delta, y, z) - self.density_at(x - delta, y, z);
    let dy = self.density_at(x, y + delta, z) - self.density_at(x, y - delta, z);
    let dz = self.density_at(x, y, z + delta) - self.density_at(x, y, z - delta);

    let v = Vector3::new(dx, dy, dz);

    // Negate because we're leaving the volume when density is *decreasing*.
    let v = -v.normalize();
    v
  }
}
