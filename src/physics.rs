use nalgebra::Vec3;
use ncollide::bounding_volume::{AABB, AABB3};
use octree::Octree;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Physics<T> {
  pub octree: Octree<T>,
  pub bounds: HashMap<T, AABB3>,
}

impl<T: Copy + Eq + PartialOrd + Hash> Physics<T> {
  pub fn insert(&mut self, t: T, bounds: &AABB3) {
    self.octree.insert(bounds.clone(), t);
    self.bounds.insert(t, bounds.clone());
  }

  pub fn remove(&mut self, t: T) {
    match self.bounds.get(&t) {
      None => {},
      Some(bounds) => {
        self.octree.remove(t, bounds);
      },
    }
  }

  pub fn get_bounds(&self, t: T) -> Option<&AABB3> {
    self.bounds.get(&t)
  }

  pub fn reinsert(octree: &mut Octree<T>, t: T, bounds: &mut AABB3, new_bounds: AABB3) -> Option<(AABB3, T)> {
    match octree.intersect(&new_bounds, Some(t)) {
      None => {
        octree.reinsert(t, bounds, new_bounds);
        *bounds = new_bounds;
        None
      },
      collision => collision,
    }
  }

  pub fn translate(&mut self, t: T, amount: Vec3<f32>) -> Option<(AABB3, T)> {
    let bounds = self.bounds.get_mut(&t).unwrap();
    let new_bounds =
      AABB::new(
        *bounds.mins() + amount,
        *bounds.maxs() + amount,
      );
    Physics::reinsert(&mut self.octree, t, bounds, new_bounds)
  }
}
