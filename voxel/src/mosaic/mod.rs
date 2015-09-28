//! A density field that also defines materials. This does not need to be defined everywhere.

use cgmath::{Point3};
use std::ops::{Deref};

pub mod solid;
pub mod union;
pub mod translation;

use field;

#[allow(missing_docs)]
pub trait T: field::T {
  #[allow(missing_docs)]
  type Material;

  /// The material density at a given point. This should be nonnegative!
  fn density(&self, p: &Point3<f32>) -> f32 {
    field::T::density(self, p).abs()
  }

  /// The material at this point.
  fn material(&self, p: &Point3<f32>) -> Option<Self::Material>;
}

impl<X: ?Sized> T for Box<X> where X: T {
  type Material = X::Material;

  fn density(&self, p: &Point3<f32>) -> f32 {
    T::density(self.deref(), p)
  }

  fn material(&self, p: &Point3<f32>) -> Option<X::Material> {
    T::material(self.deref(), p)
  }
}
