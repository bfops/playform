use edge;
use terrain_mesh;

pub struct T<Edge> {
  edges: edge::map::T<Edge>,
}

impl<Edge> T<Edge> {
  pub fn insert(&mut self, edge: edge::T, data: Edge) -> Vec<Edge> {
    let mut removed = Vec::new();
    for i in 0 .. terrain_mesh::LOD_COUNT {
      let lg_size = terrain_mesh::LG_SAMPLE_SIZE[i];
      let lg_ratio = lg_size - edge.lg_size;
      let mut edge = edge.clone();
      edge.lg_size = lg_size;
      if lg_ratio < 0 {
        edge.low_corner.x = edge.low_corner.x >> -lg_ratio;
        edge.low_corner.y = edge.low_corner.y >> -lg_ratio;
        edge.low_corner.z = edge.low_corner.z >> -lg_ratio;
      } else {
        edge.low_corner.x = edge.low_corner.x << lg_ratio;
        edge.low_corner.y = edge.low_corner.y << lg_ratio;
        edge.low_corner.z = edge.low_corner.z << lg_ratio;
      }

      self.remove(&edge)
        .map(|edge| removed.push(edge));
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
}

pub fn new<Edge>() -> T<Edge> {
  T {
    edges: edge::map::new(),
  }
}

