use cgmath::{Vector3};
use collision::{Aabb3};

use common::entity_id;
use common::fnv_map;

use octree::Octree;

pub struct T {
  pub terrain_octree : Octree<entity_id::T>,
  pub misc_octree    : Octree<entity_id::T>,
  bounds             : fnv_map::T<entity_id::T, Aabb3<f32>>,
}

pub enum Collision {
  Misc(entity_id::T),
  Terrain(entity_id::T),
}

impl T {
  pub fn new(world_bounds: Aabb3<f32>) -> T {
    T {
      terrain_octree : Octree::new(&world_bounds),
      misc_octree    : Octree::new(&world_bounds),
      bounds         : fnv_map::new(),
    }
  }

  pub fn insert_terrain(&mut self, id: entity_id::T, bounds: &Aabb3<f32>) {
    self.terrain_octree.insert(bounds, id);
    self.bounds.insert(id, *bounds);
  }

  pub fn insert_misc(&mut self, id: entity_id::T, bounds: &Aabb3<f32>) {
    self.misc_octree.insert(bounds, id);
    self.bounds.insert(id, *bounds);
  }

  pub fn remove_terrain(&mut self, id: entity_id::T) {
    match self.bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.terrain_octree.remove(bounds, id);
      },
    }
  }

  pub fn remove_misc(&mut self, id: entity_id::T) {
    match self.bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.misc_octree.remove(bounds, id);
      },
    }
  }

  pub fn get_bounds(&self, id: entity_id::T) -> Option<&Aabb3<f32>> {
    self.bounds.get(&id)
  }

  pub fn translate_misc(&mut self, id: entity_id::T, amount: Vector3<f32>) -> Option<(Aabb3<f32>, Collision)> {
    let bounds = self.bounds.get_mut(&id).unwrap();
    let new_bounds =
      Aabb3::new(
        bounds.min + amount,
        bounds.max + amount,
      );
    match self.terrain_octree.intersect(&new_bounds, None) {
      Some((bounds, terrain_id)) => {
        Some((bounds, Collision::Terrain(terrain_id)))
      },
      None => {
        match self.misc_octree.intersect(&new_bounds, Some(id)) {
          Some((bounds, misc_id)) => {
            Some ((bounds, Collision::Misc(misc_id)))
          },
          None => {
            self.misc_octree.reinsert(id, bounds, &new_bounds);
            *bounds = new_bounds;
            None
          },
        }
      },
    }
  }
}
