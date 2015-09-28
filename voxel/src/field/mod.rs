//! A density field defining a density and normal everywhere.

use cgmath::{Point3, Vector3};
use std::ops::Deref;

pub mod sphere;
pub mod intersection;
pub mod rotation;
pub mod translation;

#[allow(missing_docs)]
pub trait T {
  /// The density of the material at this point.
  fn density(&self, p: &Point3<f32>) -> f32;

  /// The surface normal at a given point.
  fn normal(&self, p: &Point3<f32>) -> Vector3<f32>;
}

impl<X: ?Sized> T for Box<X> where X: T {
  fn density(&self, p: &Point3<f32>) -> f32 {
    T::density(self.deref(), p)
  }

  fn normal(&self, p: &Point3<f32>) -> Vector3<f32> {
    T::normal(self.deref(), p)
  }
}
