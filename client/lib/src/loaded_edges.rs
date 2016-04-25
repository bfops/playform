use cgmath::{Point3, Vector3, Point};
use stopwatch;

use common::fnv_map;
use common::voxel;

use edge;

pub struct T<V> {
  edges: Vector3<voxel::storage::T<V>>,
}

fn bounds(edge: &edge::T) -> voxel::bounds::T {
  voxel::bounds::new(edge.low_corner.x, edge.low_corner.y, edge.low_corner.z, edge.lg_size)
}

// Maintain the invariant that colliding edges can't exist simultaneously in the set.
// TODO: Check this occasionally.
impl<V> T<V> {
  fn tree(&self, direction: edge::Direction) -> &voxel::storage::T<V> {
    match direction {
      edge::Direction::X => &self.edges.x,
      edge::Direction::Y => &self.edges.y,
      edge::Direction::Z => &self.edges.z,
    }
  }

  fn tree_mut(&mut self, direction: edge::Direction) -> &mut voxel::storage::T<V> {
    match direction {
      edge::Direction::X => &mut self.edges.x,
      edge::Direction::Y => &mut self.edges.y,
      edge::Direction::Z => &mut self.edges.z,
    }
  }

  #[inline(never)]
  pub fn insert(&mut self, edge: &edge::T, edge_data: V) -> Vec<V> {
    let mut removed = Vec::new();
    for collision in self.find_collisions(&edge) {
      removed.push(self.remove(&collision).unwrap());
    }

    let bounds = bounds(&edge);
    match self.tree_mut(edge.direction).entry(&bounds) {
      fnv_map::Entry::Occupied(mut entry) => {
        entry.insert(edge_data);
      },
      fnv_map::Entry::Vacant(entry) => {
        entry.insert(edge_data);
      },
    }

    removed
  }

  pub fn remove(&mut self, edge: &edge::T) -> Option<V> {
    let mut edges = self.tree_mut(edge.direction);
    let bounds = bounds(&edge);
    edges.remove(&bounds)
  }

  pub fn contains_key(&self, edge: &edge::T) -> bool {
    let edges = self.tree(edge.direction);
    let bounds = bounds(&edge);

    edges.get(&bounds).is_some()
  }

  pub fn find_collisions(&self, edge: &edge::T) -> Vec<edge::T> {
    let bounds = bounds(&edge);

    let mut collisions = Vec::new();

    {
      let mut check_collision = |lg_size, by_position: &voxel::storage::ByPosition<V>, point| {
        let b =
          stopwatch::time("loaded_edges::find_collision::get", || {
            by_position.get(&point).is_some()
          });
        if b {
          collisions.push(
            edge::T {
              low_corner: point,
              lg_size: lg_size,
              direction: edge.direction,
            }
          );
        }
        b
      };

      for &(lg_size, ref by_position) in &self.tree(edge.direction).by_lg_size {
        let lg_ratio = bounds.lg_size - lg_size;
        if lg_ratio < 0 {
          let lg_ratio = -lg_ratio;
          let found_collision =
            check_collision(
              lg_size,
              by_position,
              Point3::new(
                bounds.x >> lg_ratio,
                bounds.y >> lg_ratio,
                bounds.z >> lg_ratio,
              ),
            );
          if found_collision {
            break
          }
        } else {
          let count = 1 << lg_ratio;
          let point =
            Point3::new(
              bounds.x << lg_ratio,
              bounds.y << lg_ratio,
              bounds.z << lg_ratio,
            );
          for dx in 0..count {
          for dy in 0..count {
          for dz in 0..count {
            check_collision(lg_size, by_position, point.add_v(&Vector3::new(dx, dy, dz)));
          }}}
        }
      }
    }

    collisions
  }
}

pub fn new<V>() -> T<V> {
  T {
    edges:
      Vector3::new(
        voxel::storage::new(),
        voxel::storage::new(),
        voxel::storage::new(),
      ),
  }
}
