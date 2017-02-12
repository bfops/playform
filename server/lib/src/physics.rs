use cgmath::{Vector3};
use collision::{Aabb3};

use common::fnv_map;

use entity;
use octree::Octree;

pub struct T {
  pub terrain_octree: Octree<entity::id::Misc>,
  pub misc_octree: Octree<entity::id::Misc>,
  pub bounds: fnv_map::T<entity::id::Misc, Aabb3<f32>>,
}

impl T {
  pub fn new(world_bounds: Aabb3<f32>) -> T {
    T {
      terrain_octree : Octree::new(&world_bounds),
      misc_octree    : Octree::new(&world_bounds),
      bounds         : fnv_map::new(),
    }
  }

  pub fn insert_terrain(&mut self, id: entity::id::Misc, bounds: &Aabb3<f32>) {
    self.terrain_octree.insert(bounds, id);
    self.bounds.insert(id, *bounds);
  }

  pub fn insert_misc(&mut self, id: entity::id::Misc, bounds: &Aabb3<f32>) {
    self.misc_octree.insert(bounds, id);
    self.bounds.insert(id, *bounds);
  }

  pub fn remove_terrain(&mut self, id: entity::id::Misc) {
    match self.bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.terrain_octree.remove(bounds, id);
      },
    }
  }

  pub fn remove_misc(&mut self, id: entity::id::Misc) {
    match self.bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.misc_octree.remove(bounds, id);
      },
    }
  }

  pub fn get_bounds(&self, id: entity::id::Misc) -> Option<&Aabb3<f32>> {
    self.bounds.get(&id)
  }

  pub fn reinsert(
    octree: &mut Octree<entity::id::Misc>,
    id: entity::id::Misc,
    bounds: &mut Aabb3<f32>,
    new_bounds: &Aabb3<f32>,
  ) -> Option<(Aabb3<f32>, entity::id::Misc)> {
    match octree.intersect(new_bounds, Some(id)) {
      None => {
        octree.reinsert(id, bounds, new_bounds);
        *bounds = *new_bounds;
        None
      },
      collision => collision,
    }
  }

  pub fn translate_misc(&mut self, id: entity::id::Misc, amount: Vector3<f32>) -> Option<(Aabb3<f32>, entity::id::Misc)> {
    let bounds = self.bounds.get_mut(&id).unwrap();
    let new_bounds =
      Aabb3::new(
        bounds.min + (&amount),
        bounds.max + (&amount),
      );
    let terrain_collision = self.terrain_octree.intersect(&new_bounds, None);
    if terrain_collision.is_none() {
      T::reinsert(&mut self.misc_octree, id, bounds, &new_bounds)
    } else {
      terrain_collision
    }
  }
}
