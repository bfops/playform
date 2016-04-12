use std;
use cgmath::Vector3;

use common::fnv_map;
use common::voxel;

use edge;

pub struct T<V> {
  edges: Vector3<voxel::storage::T<(edge::T, V)>>,
}

fn bounds(edge: &edge::T) -> voxel::bounds::T {
  voxel::bounds::new(edge.low_corner.x, edge.low_corner.y, edge.low_corner.z, edge.lg_size)
}

impl<V> T<V> {
  fn tree(&self, direction: edge::Direction) -> &voxel::storage::T<(edge::T, V)> {
    match direction {
      edge::Direction::X => &self.edges.x,
      edge::Direction::Y => &self.edges.y,
      edge::Direction::Z => &self.edges.z,
    }
  }

  fn tree_mut(&mut self, direction: edge::Direction) -> &mut voxel::storage::T<(edge::T, V)> {
    match direction {
      edge::Direction::X => &mut self.edges.x,
      edge::Direction::Y => &mut self.edges.y,
      edge::Direction::Z => &mut self.edges.z,
    }
  }

  pub fn insert(&mut self, edge: &edge::T, edge_data: V) -> Vec<V> {
    let mut removed = Vec::new();
    for collision in self.find_collisions(&edge) {
      removed.push(self.remove(&collision).unwrap());
    }

    let bounds = bounds(&edge);
    match self.tree_mut(edge.direction).entry(&bounds) {
      fnv_map::Entry::Occupied(mut entry) => entry.insert((*edge, edge_data)),
      fnv_map::Entry::Vacant(mut entry)   => entry.insert((*edge, edge_data)),
    }

    removed
  }

  pub fn remove(&mut self, edge: &edge::T) -> Option<V> {
    let mut edges = self.tree_mut(edge.direction);
    let bounds = bounds(&edge);
    edges.remove(&bounds).map(|(_, v)| v)
  }

  pub fn contains_key(&self, edge: &edge::T) -> bool {
    let edges = self.tree(edge.direction);
    let bounds = bounds(&edge);

    edges.get(&bounds).is_some()
  }

  pub fn find_collisions(&self, edge: &edge::T) -> Vec<edge::T> {
    fn all<V>(collisions: &mut Vec<edge::T>, branches: &voxel::storage::Inner<(edge::T, V)>) {
      if let &voxel::storage::Inner::Branches(ref branches) = branches {
        if let Some((edge, _)) = branches.data {
          collisions.push(edge);
        }

        for branches in branches.as_flat_array() {
          all(collisions, branches);
        }
      }
    }

    let mut collisions = Vec::new();

    let bounds = bounds(edge);
    let edges = self.tree(edge.direction);
    let mut traversal = voxel::storage::traversal::to_voxel(&edges, &bounds);
    let mut edges = &edges.contents;
    loop {
      match traversal.next(edges) {
        voxel::storage::traversal::Step::Last(branches) => {
          all(&mut collisions, branches);
          break;
        },
        voxel::storage::traversal::Step::Step(branches) => {
          if let &voxel::storage::Inner::Branches(ref branches) = branches {
            if let Some((edge, _)) = branches.data {
              collisions.push(edge);
            }
            edges = branches;
          } else {
            break;
          }
        },
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
