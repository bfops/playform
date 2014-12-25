use nalgebra::Vec3;
use ncollide::bounding_volume::{AABB, AABB3};
use octree::Octree;
use state::EntityId;
use std::collections::HashMap;

pub struct Physics {
  pub terrain_octree: Octree<EntityId>,
  pub misc_octree: Octree<EntityId>,
  pub bounds: HashMap<EntityId, AABB3<f32>>,
}

impl Physics {
  pub fn new(world_bounds: AABB3<f32>) -> Physics {
    Physics {
      terrain_octree: Octree::new(&world_bounds),
      misc_octree: Octree::new(&world_bounds),
      bounds: HashMap::new(),
    }
  }

  pub fn insert_terrain(&mut self, id: EntityId, bounds: &AABB3<f32>) {
    self.terrain_octree.insert(bounds.clone(), id);
    self.bounds.insert(id, bounds.clone());
  }

  pub fn insert_misc(&mut self, id: EntityId, bounds: &AABB3<f32>) {
    self.misc_octree.insert(bounds.clone(), id);
    self.bounds.insert(id, bounds.clone());
  }

  pub fn remove_terrain(&mut self, id: EntityId) {
    match self.bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.terrain_octree.remove(bounds, id);
      },
    }
  }

  pub fn remove_misc(&mut self, id: EntityId) {
    match self.bounds.get(&id) {
      None => {},
      Some(bounds) => {
        self.misc_octree.remove(bounds, id);
      },
    }
  }

  pub fn get_bounds(&self, id: EntityId) -> Option<&AABB3<f32>> {
    self.bounds.get(&id)
  }

  pub fn reinsert(
    octree: &mut Octree<EntityId>,
    id: EntityId,
    bounds: &mut AABB3<f32>,
    new_bounds: AABB3<f32>,
  ) -> Option<(AABB3<f32>, EntityId)> {
    match octree.intersect(&new_bounds, Some(id)) {
      None => {
        octree.reinsert(id, bounds, new_bounds.clone());
        *bounds = new_bounds;
        None
      },
      collision => collision,
    }
  }

  pub fn translate_misc(&mut self, id: EntityId, amount: Vec3<f32>) -> Option<(AABB3<f32>, EntityId)> {
    let bounds = self.bounds.get_mut(&id).unwrap();
    let new_bounds =
      AABB::new(
        *bounds.mins() + amount,
        *bounds.maxs() + amount,
      );
    let terrain_collision = self.terrain_octree.intersect(&new_bounds, None);
    if terrain_collision.is_none() {
      Physics::reinsert(&mut self.misc_octree, id, bounds, new_bounds)
    } else {
      terrain_collision
    }
  }
}
