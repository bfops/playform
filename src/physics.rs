use nalgebra::Vec3;
use ncollide::bounding_volume::aabb::AABB;
use ncollide::math::Scalar;
use octree::Octree;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Physics<T> {
  pub octree: Octree<T>,
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

  pub fn translate(&mut self, t: T, amount: Vec3<Scalar>) -> Option<(AABB, T)> {
    let bounds = self.bounds.find_mut(&t).unwrap();
    let new_bounds =
      AABB::new(
        bounds.mins() + amount,
        bounds.maxs() + amount
      );

    match self.octree.intersect(&new_bounds, Some(t)) {
      None => {
        self.octree.reinsert(t, bounds, new_bounds);
        *bounds = new_bounds;
        None
      },
      collision => collision,
    }
  }
}
