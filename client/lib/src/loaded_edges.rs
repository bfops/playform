use cgmath::{Point3, Vector3};

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
      let is_collision = |by_position: &voxel::storage::ByPosition<V>, point| {
        by_position.get(&point).is_some()
      };

      let construct_collision = |lg_size, point| {
        edge::T {
          low_corner: point,
          lg_size: lg_size,
          direction: edge.direction,
        }
      };

      let tree = self.tree(edge.direction);

      let by_lg_size = &tree.by_lg_size;
      let mut lg_size_indices: Vec<_> = by_lg_size.iter().map(|&(x, _)| x).enumerate().collect();
      lg_size_indices.sort_by_key(|&(_, x)| -x);
      let lg_size_indices: Vec<usize> = lg_size_indices.into_iter().map(|(i, _)| i).collect();

      let mut i = 0;
      while i < lg_size_indices.len() {
        let &(lg_size, ref by_position) = &by_lg_size[ lg_size_indices[i] ];

        let lg_ratio = bounds.lg_size - lg_size;
        if lg_ratio > 0 {
          break
        }

        let lg_ratio = -lg_ratio;
        let coord =
          Point3::new(
            bounds.x >> lg_ratio,
            bounds.y >> lg_ratio,
            bounds.z >> lg_ratio,
          );
        let is_collision = is_collision(by_position, coord);
        if is_collision {
          collisions.push(construct_collision(lg_size, coord));
          return collisions
        }

        i += 1;
      }

      all(edge.direction, &mut collisions, &lg_size_indices, by_lg_size, i, bounds);
    }

    collisions
  }
}

#[allow(warnings)]
fn all<V>(
  direction: edge::Direction,
  collisions: &mut Vec<edge::T>,
  lg_size_indices: &[usize],
  by_lg_size: &voxel::storage::ByLgSize<voxel::storage::ByPosition<V>>,
  i: usize,
  bounds: voxel::bounds::T,
) {
  if i >= lg_size_indices.len() {
    return
  }

  let &(lg_size, ref by_position) = &by_lg_size[lg_size_indices[i]];

  let is_collision = |by_position: &voxel::storage::ByPosition<V>, point| {
    by_position.get(&point).is_some()
  };

  let construct_collision = |lg_size, point| {
    edge::T {
      low_corner: point,
      lg_size: lg_size,
      direction: direction,
    }
  };

  let lg_ratio = lg_size - bounds.lg_size;
  debug_assert!(lg_ratio > 0);
  let count = 1 << lg_ratio;
  let bounds =
    voxel::bounds::new(
      bounds.x << lg_ratio,
      bounds.y << lg_ratio,
      bounds.z << lg_ratio,
      lg_size,
    );
  for dx in 0..count {
  for dy in 0..count {
  for dz in 0..count {
    let point = Point3::new(bounds.x + dx, bounds.y + dy, bounds.z + dz);
    if is_collision(by_position, point) {
      collisions.push(construct_collision(lg_size, point));
    } else {
      let bounds = voxel::bounds::new(point.x, point.y, point.z, lg_size);
      all(direction, collisions, lg_size_indices, by_lg_size, i + 1, bounds);
    }
  }}}
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
