use nalgebra::na::Vec3;
use ncollide3df32::bounding_volume::aabb::AABB;
use ncollide3df32::math::Scalar;
use octree;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Physics<T> {
  pub octree: octree::Octree<T>,
  pub locations: HashMap<T, *mut octree::Octree<T>>,
  pub bounds: HashMap<T, AABB>,
}

impl<T: Copy + Eq + PartialOrd + Hash> Physics<T> {
  pub fn insert(&mut self, t: T, bounds: &AABB) {
    let loc = self.octree.insert(bounds.clone(), t);
    self.locations.insert(t, loc);
    self.bounds.insert(t, bounds.clone());
  }

  pub fn remove(&mut self, t: T) {
    match self.bounds.find(&t) {
      None => {},
      Some(bounds) => {
        let &octree_location = self.locations.find(&t).expect("no octree_location to remove");
        self.locations.remove(&t);
        unsafe {
          (*octree_location).remove(t, bounds);
        }
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

        let octree_location = *(self.locations.find(&t).expect("location prematurely deleted"));
        assert!(octree_location.is_not_null());
        let collision = unsafe {
          (*octree_location).intersect(&new_bounds, t)
        };

        if !collision {
          unsafe {
            let octree_location = (*octree_location).move(t, bounds, new_bounds);
            self.locations.insert(t, octree_location);
            *bounds = new_bounds;
          }
        }

        Some(collision)
      },
    }
  }
}
