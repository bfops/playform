//! Data structure for a small block of terrain.

use cgmath::{Point, Point3, Vector3, Aabb, Aabb3};
use isosurface_extraction::dual_contouring;
use num::iter::range_inclusive;
use std::f32;
use std::collections::HashMap;
use std::sync::Mutex;
use stopwatch;
use voxel_data;

use common::entity_id;
use common::id_allocator;
use common::voxel;
// TODO: Move the server-only parts to the server, like BLOCK_WIDTH and sample_info.

use block_position;
use lod;

/// Number of LODs
pub const LOD_COUNT: usize = 4;
/// lg(WIDTH)
pub const LG_WIDTH: i16 = 3;
/// The width of a block of terrain.
pub const WIDTH: i32 = 1 << LG_WIDTH;

/// lg(EDGE_SAMPLES)
pub const LG_EDGE_SAMPLES: [u16; LOD_COUNT] = [3, 2, 1, 0];
/// The number of voxels along an axis within a block, indexed by LOD.
pub const EDGE_SAMPLES: [u16; LOD_COUNT] = [
  1 << LG_EDGE_SAMPLES[0],
  1 << LG_EDGE_SAMPLES[1],
  1 << LG_EDGE_SAMPLES[2],
  1 << LG_EDGE_SAMPLES[3],
];

/// The width of a voxel within a block, indexed by LOD.
pub const LG_SAMPLE_SIZE: [i16; LOD_COUNT] = [
  LG_WIDTH - LG_EDGE_SAMPLES[0] as i16,
  LG_WIDTH - LG_EDGE_SAMPLES[1] as i16,
  LG_WIDTH - LG_EDGE_SAMPLES[2] as i16,
  LG_WIDTH - LG_EDGE_SAMPLES[3] as i16,
];

#[derive(Debug, Copy, Clone, RustcEncodable, RustcDecodable)]
/// [T; 3], but serializable.
pub struct Triangle<T> {
  #[allow(missing_docs)]
  pub v1: T,
  #[allow(missing_docs)]
  pub v2: T,
  #[allow(missing_docs)]
  pub v3: T,
}

/// Construct a triangle.
pub fn tri<T>(v1: T, v2: T, v3: T) -> Triangle<T> {
  Triangle {
    v1: v1,
    v2: v2,
    v3: v3,
  }
}

fn make_bounds(
  v1: &Point3<f32>,
  v2: &Point3<f32>,
  v3: &Point3<f32>,
) -> Aabb3<f32> {
  let minx = f32::min(v1.x, f32::min(v2.x, v3.x));
  let maxx = f32::max(v1.x, f32::max(v2.x, v3.x));

  let miny = f32::min(v1.y, f32::min(v2.y, v3.y));
  let maxy = f32::max(v1.y, f32::max(v2.y, v3.y));

  let minz = f32::min(v1.z, f32::min(v2.z, v3.z));
  let maxz = f32::max(v1.z, f32::max(v2.z, v3.z));

  Aabb3::new(
    // TODO: Remove this - 1.0. It's a temporary hack until voxel collisions work,
    // to avoid zero-height Aabb3s.
    Point3::new(minx, miny - 1.0, minz),
    Point3::new(maxx, maxy, maxz),
  )
}

pub fn voxels_in(bounds: &Aabb3<i32>, lg_size: i16) -> Vec<voxel_data::bounds::T> {
  let delta = bounds.max().sub_p(bounds.min());

  // assert that lg_size samples fit neatly into the bounds.
  let mod_size = (1 << lg_size) - 1;
  assert!(bounds.min().x & mod_size == 0);
  assert!(bounds.min().y & mod_size == 0);
  assert!(bounds.min().z & mod_size == 0);
  assert!(bounds.max().x & mod_size == 0);
  assert!(bounds.max().y & mod_size == 0);
  assert!(bounds.max().z & mod_size == 0);

  let mut voxels = Vec::new();
  for dx in 0 .. delta.x >> lg_size {
  for dy in 0 .. delta.y >> lg_size {
  for dz in 0 .. delta.z >> lg_size {
    let x = (bounds.min().x >> lg_size) + dx;
    let y = (bounds.min().y >> lg_size) + dy;
    let z = (bounds.min().z >> lg_size) + dz;
    voxels.push(voxel_data::bounds::new(x, y, z, lg_size));
  }}}
  voxels
}

mod voxel_storage {
  use voxel_data;
  use isosurface_extraction::dual_contouring;

  use common::voxel;

  use super::Partial;

  fn get_voxel<'a>(this: &'a mut Partial, bounds: &voxel_data::bounds::T) -> Option<&'a voxel::T> {
    debug!("Getting {:?} from {:?}", bounds, this.position);
    this.voxels.get(bounds)
  }

  impl<'a> dual_contouring::voxel_storage::T<voxel::Material> for Partial {
    fn get_material(&mut self, bounds: &voxel_data::bounds::T) -> Option<voxel::Material> {
      match get_voxel(self, bounds) {
        None => None,
        Some(&voxel::Surface(ref voxel)) => Some(voxel.corner.clone()),
        Some(&voxel::Volume(ref material)) => Some(material.clone()),
      }
    }

    fn get_voxel_data(&mut self, bounds: &voxel_data::bounds::T) -> Option<dual_contouring::voxel_storage::VoxelData> {
      match get_voxel(self, bounds) {
        None => None,
        Some(&voxel::Volume(_)) => panic!("Can't extract voxel data from a volume"),
        Some(&voxel::Surface(ref voxel)) =>
          Some({
            dual_contouring::voxel_storage::VoxelData {
              bounds: bounds.clone(),
              vertex: voxel.surface_vertex.to_world_vertex(&bounds),
              normal: voxel.normal.to_float_normal(),
            }
          })
      }
    }
  }
}

pub struct Partial {
  position: block_position::T,
  lod: lod::T,
  voxels: HashMap<voxel_data::bounds::T, voxel::T>,
}

impl Partial {
  pub fn new(position: block_position::T, lod: lod::T) -> Partial {
    Partial {
      position: position,
      lod: lod,
      voxels: HashMap::new(),
    }
  }

  pub fn lod(&self) -> lod::T {
    self.lod
  }

  pub fn add(&mut self, voxel: voxel::T, bounds: voxel_data::bounds::T) {
    self.voxels.insert(bounds, voxel);
  }

  pub fn finalize(
    &mut self,
    id_allocator: &Mutex<id_allocator::T<entity_id::T>>,
  ) -> Option<T>
  {
    // We load a buffer of one voxel around each chunk so we can render seams.
    // TODO: PartialBlock::voxels_to_fetch();
    let edge_samples = EDGE_SAMPLES[self.lod.0 as usize] as usize + 2;
    let samples = edge_samples * edge_samples * edge_samples;
    assert!(self.voxels.len() <= samples);
    trace!("len {:?} out of {:?}", self.voxels.len(), samples);
    if self.voxels.len() < samples {
      return None
    }

    stopwatch::time("terrain_mesh::finalize", || {
      let mut block = empty();

      let lg_edge_samples = LG_EDGE_SAMPLES[self.lod.0 as usize];
      let lg_sample_size = LG_SAMPLE_SIZE[self.lod.0 as usize];

      let low = self.position.as_pnt().clone();
      let high = low.add_v(&Vector3::new(1, 1, 1));
      let low =
        Point3::new(
          low.x << lg_edge_samples,
          low.y << lg_edge_samples,
          low.z << lg_edge_samples,
        );
      let high =
        Point3::new(
          high.x << lg_edge_samples,
          high.y << lg_edge_samples,
          high.z << lg_edge_samples,
        );

      trace!("low {:?}", low);
      trace!("high {:?}", high);

      {
        let mut edges = |direction, low_x, high_x, low_y, high_y, low_z, high_z| {
          for x in range_inclusive(low_x, high_x) {
          for y in range_inclusive(low_y, high_y) {
          for z in range_inclusive(low_z, high_z) {
            trace!("edge: {:?} {:?}", direction, Point3::new(x, y, z));
            let edge = 
              dual_contouring::edge::T {
                low_corner: Point3::new(x, y, z),
                direction: direction,
                lg_size: lg_sample_size,
              };

            dual_contouring::edge::extract(
              self,
              &edge,
              &mut |polygon: dual_contouring::polygon::T<voxel::Material>| {
                let id = id_allocator.lock().unwrap().allocate();

                block.vertex_coordinates.push(tri(polygon.vertices[0], polygon.vertices[1], polygon.vertices[2]));
                block.normals.push(tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]));
                block.materials.push(polygon.material as i32);
                block.ids.push(id);
                block.bounds.push((id, make_bounds(&polygon.vertices[0], &polygon.vertices[1], &polygon.vertices[2])));
              }
            );
          }}}
        };

        edges(
          dual_contouring::edge::Direction::X,
          low.x, high.x - 1,
          low.y, high.y - 1,
          low.z, high.z - 1,
        );
        edges(
          dual_contouring::edge::Direction::Y,
          low.x, high.x - 1,
          low.y, high.y - 1,
          low.z, high.z - 1,
        );
        edges(
          dual_contouring::edge::Direction::Z,
          low.x, high.x - 1,
          low.y, high.y - 1,
          low.z, high.z - 1,
        );
      }

      Some(block)
    })
  }
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
/// A small continguous chunk of terrain.
pub struct T {
  // These Vecs must all be ordered the same way; each entry is the next triangle.

  /// Position of each vertex.
  pub vertex_coordinates: Vec<Triangle<Point3<f32>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<Triangle<Vector3<f32>>>,
  /// Entity IDs for each triangle.
  pub ids: Vec<entity_id::T>,
  /// Material IDs for each triangle.
  pub materials: Vec<i32>,
  // TODO: Change this back to a HashMap once initial capacity is zero for those.
  /// Per-triangle bounding boxes.
  pub bounds: Vec<(entity_id::T, Aabb3<f32>)>,
}

/// Construct an empty `T`.
pub fn empty() -> T {
  T {
    vertex_coordinates: Vec::new(),
    normals: Vec::new(),

    ids: Vec::new(),
    materials: Vec::new(),
    bounds: Vec::new(),
  }
}
