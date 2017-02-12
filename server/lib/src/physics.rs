use cgmath::{Vector3};
use collision::{Aabb3};

use common::fnv_map;

use entity;
use octree::Octree;

pub struct T {
  pub terrain_octree : Octree<entity::id::Terrain>,
  terrain_bounds : fnv_map::T<entity::id::Terrain, Aabb3<f32>>,
  pub misc_octree    : Octree<entity::id::Misc>,
  misc_bounds    : fnv_map::T<entity::id::Misc, Aabb3<f32>>,
}

pub enum Collision {
  Terrain(entity::id::Terrain),
  Misc(entity::id::Misc),
}

impl T {
  pub fn new(world_bounds: Aabb3<f32>) -> T {
    T {
      terrain_octree : Octree::new(&world_bounds),
      terrain_bounds : fnv_map::new(),
      misc_octree    : Octree::new(&world_bounds),
      misc_bounds    : fnv_map::new(),
    }
  }

  pub fn insert_terrain(&mut self, id: entity::id::Terrain, bounds: &Aabb3<f32>) {
    self.terrain_octree.insert(bounds, id);
    self.terrain_bounds.insert(id, *bounds);
  }

  pub fn insert_misc(&mut self, id: entity::id::Misc, bounds: &Aabb3<f32>) {
    self.misc_octree.insert(bounds, id);
    self.misc_bounds.insert(id, *bounds);
  }

  pub fn remove_terrain(&mut self, id: entity::id::Terrain) {
    match self.terrain_bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.terrain_octree.remove(bounds, id);
      },
    }
  }

  pub fn remove_misc(&mut self, id: entity::id::Misc) {
    match self.misc_bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.misc_octree.remove(bounds, id);
      },
    }
  }

  pub fn get_bounds(&self, id: entity::id::Misc) -> Option<&Aabb3<f32>> {
    self.misc_bounds.get(&id)
  }

  pub fn get_mut_bounds(&mut self, id: entity::id::Misc) -> Option<&mut Aabb3<f32>> {
    self.misc_bounds.get_mut(&id)
  }

  pub fn translate_misc(&mut self, id: entity::id::Misc, amount: Vector3<f32>) -> Option<(Aabb3<f32>, Collision)> {
    let bounds = self.misc_bounds.get_mut(&id).unwrap();
    let new_bounds =
      Aabb3::new(
        bounds.min + amount,
        bounds.max + amount,
      );
    match self.terrain_octree.intersect(&new_bounds, None) {
      None => {
        match self.misc_octree.intersect(&new_bounds, Some(id)) {
          None => {
            self.misc_octree.reinsert(id, bounds, &new_bounds);
            *bounds = new_bounds;
            None
          },
          Some((bounds, misc_id)) => Some ((bounds, Collision::Misc(misc_id))),
        }
      },
      Some((bounds, terrain_id)) => {
        Some((bounds, Collision::Terrain(terrain_id)))
      },
    }
  }
}
