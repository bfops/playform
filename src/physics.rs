use nalgebra::na::Vec3;
use ncollide3df32::bounding_volume::aabb::AABB;
use ncollide3df32::math::Scalar;
use octree;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Physics<T> {
  pub octree: octree::Octree<T>,
  pub bounds: HashMap<T, AABB>,
}

impl<T: Copy + Eq + PartialOrd + Hash> Physics<T> {
  pub fn insert(&mut self, t: T, bounds: &AABB) {
    self.octree.insert(bounds.clone(), t);
    self.bounds.insert(t, bounds.clone());
  }

  pub fn remove(&mut self, t: T) {
    match self.bounds.find(&t) {
      None => {},
      Some(bounds) => {
        self.octree.remove(t, bounds);
      },
    }
  }

  pub fn get_bounds(&self, t: T) -> Option<&AABB> {
    self.bounds.find(&t)
  }

  pub fn translate(&mut self, t: T, amount: Vec3<Scalar>) -> Option<bool> {
    match self.bounds.find_mut(&t) {
      None => None,
      Some(bounds) => {
        let new_bounds =
          AABB::new(
            bounds.mins() + amount,
            bounds.maxs() + amount
          );

        let collision = self.octree.intersect(&new_bounds, t);

        if !collision {
          self.octree.move(t, bounds, new_bounds);
          *bounds = new_bounds;
        }

        Some(collision)
      },
    }
  }
}
