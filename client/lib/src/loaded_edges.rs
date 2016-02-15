use std;
use cgmath::{Point, Vector, Vector3};
use voxel_data;

use edge;
use terrain_mesh;

pub struct T<Edge> {
  edges: Vector3<voxel_data::tree::T<(edge::T, Edge)>>,
}

fn bounds(edge: &edge::T) -> voxel_data::bounds::T {
  voxel_data::bounds::new(edge.low_corner.x, edge.low_corner.y, edge.low_corner.z, edge.lg_size)
}

impl<Edge> T<Edge> {
  fn tree(&self, direction: edge::Direction) -> &voxel_data::tree::T<(edge::T, Edge)> {
    match direction {
      edge::Direction::X => &self.edges.x,
      edge::Direction::Y => &self.edges.y,
      edge::Direction::Z => &self.edges.z,
    }
  }

  fn tree_mut(&mut self, direction: edge::Direction) -> &mut voxel_data::tree::T<(edge::T, Edge)> {
    match direction {
      edge::Direction::X => &mut self.edges.x,
      edge::Direction::Y => &mut self.edges.y,
      edge::Direction::Z => &mut self.edges.z,
    }
  }

  pub fn insert(&mut self, edge: &edge::T, edge_data: Edge) -> Vec<Edge> {
    let bounds = bounds(&edge);

    let mut removed = Vec::new();
    for edge in self.find_collisions(edge) {
      removed.push(self.remove(&edge).unwrap());
    }

    let edges = self.tree_mut(edge.direction);

    edges
      .get_mut_or_create(&bounds)
      .force_branches()
      .data = Some((*edge, edge_data));

    removed
  }

  pub fn remove(&mut self, edge: &edge::T) -> Option<Edge> {
    let mut edges = self.tree_mut(edge.direction);
    let bounds = bounds(&edge);

    match edges.get_mut_pointer(&bounds) {
      Some(&mut voxel_data::tree::Inner::Branches(ref mut branches)) => {
        let mut r = None;
        std::mem::swap(&mut branches.data, &mut r);
        r.map(|(_, d)| d)
      },
      _ => None,
    }
  }

  pub fn contains_key(&self, edge: &edge::T) -> bool {
    let edges = self.tree(edge.direction);
    let bounds = bounds(&edge);

    edges.get(&bounds).is_some()
  }

  pub fn find_collisions(&self, edge: &edge::T) -> Vec<edge::T> {
    let mut collisions = Vec::new();
    for i in 0 .. terrain_mesh::LOD_COUNT {
      let lg_size = terrain_mesh::LG_SAMPLE_SIZE[i];
      let lg_ratio = lg_size - edge.lg_size;

      let mut edge = *edge;
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
        let mut edge = edge;
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
    edges:
      Vector3::new(
        voxel_data::tree::new(),
        voxel_data::tree::new(),
        voxel_data::tree::new(),
      ),
  }
}
