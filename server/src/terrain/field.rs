use cgmath::{EuclideanVector, Point3, Vector3};

/// A trait representing a density field.
pub trait Field {
  /// The height of the field at a given x,y,z.
  fn density_at(&self, p: &Point3<f32>) -> f32;

  /// [field.contains(p)] == [field.density_at(p) >= 0.0]
  fn contains(&self, p: &Point3<f32>) -> bool {
    self.density_at(p) >= 0.0
  }

  // TODO: Should delta be part of the struct instead of the trait?
  /// The lighting normal of the tile at a given x,y,z.
  fn normal_at(&self, delta: f32, p: &Point3<f32>) -> Vector3<f32> {
    // Use density differential in each dimension as an approximation of the normal.

    macro_rules! differential(($d:ident) => {{
      let high: f32 = {
        let mut p = *p;
        p.$d += delta;
        self.density_at(&p)
      };
      let low: f32 = {
        let mut p = *p;
        p.$d -= delta;
        self.density_at(&p)
      };
      high - low
    }});

    let v = Vector3::new(differential!(x), differential!(y), differential!(z));
    // Negate because we're leaving the volume when density is decreasing.
    let v = -v;
    v.normalize()
  }
}
