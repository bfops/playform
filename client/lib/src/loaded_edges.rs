use std;
use cgmath::{Point, Vector};

use edge;
use terrain_mesh;

pub struct T<Edge> {
  edges: edge::map::T<Edge>,
}

impl<Edge> T<Edge> {
  pub fn insert(&mut self, edge: edge::T, data: Edge) -> Vec<Edge> {
    let mut removed = Vec::new();
    for edge in self.find_collisions(&edge) {
      removed.push(self.remove(&edge).unwrap());
    }

    self.edges.insert(edge, data);

    removed
  }

  pub fn remove(&mut self, edge: &edge::T) -> Option<Edge> {
    self.edges.remove(edge)
  }

  pub fn contains_key(&self, edge: &edge::T) -> bool {
    self.edges.contains_key(edge)
  }

  pub fn find_collisions(&self, edge: &edge::T) -> Vec<edge::T> {
    let mut collisions = Vec::new();
    for i in 0 .. terrain_mesh::LOD_COUNT {
      let lg_size = terrain_mesh::LG_SAMPLE_SIZE[i];
      let lg_ratio = lg_size - edge.lg_size;

      let mut edge = edge.clone();
      edge.lg_size = lg_size;
      if lg_ratio < 0 {
        edge.low_corner.x = edge.low_corner.x << -lg_ratio;
        edge.low_corner.y = edge.low_corner.y << -lg_ratio;
        edge.low_corner.z = edge.low_corner.z << -lg_ratio;
      } else {
        edge.low_corner.x = edge.low_corner.x >> lg_ratio;
        edge.low_corner.y = edge.low_corner.y >> lg_ratio;
        edge.low_corner.z = edge.low_corner.z >> lg_ratio;
      }

      for i in 0 .. (1 << std::cmp::max(0, -lg_ratio)) {
        let mut edge = edge.clone();
        edge.low_corner.add_self_v(&edge.direction.to_vec().mul_s(i));

        if self.contains_key(&edge) {
          collisions.push(edge);
        }
      }
    }

    collisions
  }
}

pub fn new<Edge>() -> T<Edge> {
  T {
    edges: edge::map::new(),
  }
}
