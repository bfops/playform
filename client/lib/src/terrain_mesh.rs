//! Data structure for a small chunk of terrain.

use cgmath::{Point3, Vector3};
use collision::{Aabb, Aabb3};
use isosurface_extraction::dual_contouring;
use num::iter::range_inclusive;
use rand;
use std::f32;
use std::sync::Mutex;
use stopwatch;

use common::entity_id;
use common::id_allocator;
use common::voxel;
// TODO: Move the server-only parts to the server, like BLOCK_WIDTH and sample_info.

use chunk;
use lod;

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

pub fn voxels_in(bounds: &Aabb3<i32>, lg_size: i16) -> Vec<voxel::bounds::T> {
  let delta = bounds.max() - (bounds.min());

  // assert that lg_size samples fit neatly into the bounds.
  let mod_size = (1 << lg_size) - 1;
  assert!(bounds.min().x & mod_size == 0);
  assert!(bounds.min().y & mod_size == 0);
  assert!(bounds.min().z & mod_size == 0);
  assert!(bounds.max().x & mod_size == 0);
  assert!(bounds.max().y & mod_size == 0);
  assert!(bounds.max().z & mod_size == 0);

  let x_len = delta.x >> lg_size;
  let y_len = delta.y >> lg_size;
  let z_len = delta.z >> lg_size;

  let x_off = bounds.min().x >> lg_size;
  let y_off = bounds.min().y >> lg_size;
  let z_off = bounds.min().z >> lg_size;

  let mut voxels =
    Vec::with_capacity(x_len as usize + y_len as usize + z_len as usize);

  for dx in 0 .. x_len {
  for dy in 0 .. y_len {
  for dz in 0 .. z_len {
    let x = x_off + dx;
    let y = y_off + dy;
    let z = z_off + dz;
    voxels.push(voxel::bounds::new(x, y, z, lg_size));
  }}}
  voxels
}

mod voxel_storage {
  use isosurface_extraction::dual_contouring;

  use common::voxel;

  pub struct T<'a> {
    pub voxels: &'a voxel::tree::T,
  }

  fn get_voxel<'a>(this: &mut T<'a>, bounds: &voxel::bounds::T) -> Option<&'a voxel::T> {
    trace!("fetching {:?}", bounds);
    Some(this.voxels.get(bounds).unwrap_or_else(|| panic!("No entry for {:?}", bounds)))
  }

  impl<'a> dual_contouring::voxel_storage::T<voxel::Material> for T<'a> {
    fn get_material(&mut self, bounds: &voxel::bounds::T) -> Option<voxel::Material> {
      match get_voxel(self, bounds) {
        None => None,
        Some(&voxel::Surface(ref voxel)) => Some(voxel.corner),
        Some(&voxel::Volume(ref material)) => Some(*material),
      }
    }

    fn get_voxel_data(&mut self, bounds: &voxel::bounds::T) -> Option<dual_contouring::voxel_storage::VoxelData> {
      match get_voxel(self, bounds) {
        None => None,
        Some(&voxel::Volume(_)) => panic!("Can't extract voxel data from a volume"),
        Some(&voxel::Surface(ref voxel)) =>
          Some({
            dual_contouring::voxel_storage::VoxelData {
              bounds: *bounds,
              vertex: voxel.surface_vertex.to_world_vertex(&bounds),
              normal: voxel.normal.to_float_normal(),
            }
          })
      }
    }
  }
}

pub fn generate<Rng: rand::Rng>(
  voxels: &voxel::tree::T,
  chunk_position: &chunk::position::T,
  lod: lod::T,
  id_allocator: &Mutex<id_allocator::T<entity_id::T>>,
  rng: &mut Rng,
) -> T
{
  stopwatch::time("terrain_mesh::generate", || {
    let chunk_id = id_allocator::allocate(id_allocator);
    let mut chunk = empty(chunk_id);
    {
      let lg_edge_samples = lod.lg_edge_samples();
      let lg_sample_size = lod.lg_sample_size();

      let low = *chunk_position.as_pnt();
      let high = low + (&Vector3::new(1, 1, 1));
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

            let _ =
              dual_contouring::edge::extract(
                &mut voxel_storage::T { voxels: voxels },
                &edge,
                &mut |polygon: dual_contouring::polygon::T<voxel::Material>| {
                  chunk.vertex_coordinates.push(tri(polygon.vertices[0], polygon.vertices[1], polygon.vertices[2]));
                  chunk.normals.push(tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]));
                  chunk.materials.push(polygon.material as i32);

                  if polygon.material == voxel::Material::Terrain && lod <= lod::MAX_GRASS_LOD {
                    chunk.grass.push(
                      Grass {
                        tex_id: rng.gen_range(0, 9),
                      }
                    );
                    chunk.ids.grass_ids.push(id_allocator::allocate(id_allocator));
                  }
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
    }
    chunk
  })
}

#[derive(Debug, Clone)]
pub struct Grass {
  pub tex_id : u32,
}

#[derive(Debug, Clone)]
pub struct Ids {
  /// Entity ID for the terrain buffers.
  pub chunk_id: entity_id::T,
  /// Entity IDs for each grass tuft.
  pub grass_ids: Vec<entity_id::T>,
}

impl Ids {
  pub fn new(chunk_id: entity_id::T) -> Self {
    Ids {
      chunk_id  : chunk_id,
      grass_ids : Vec::new(),
    }
  }
}

#[derive(Debug)]
/// A small continguous chunk of terrain.
pub struct T {
  // These Vecs must all be ordered the same way; each entry is the next triangle.

  /// Position of each vertex.
  pub vertex_coordinates: Vec<Triangle<Point3<f32>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<Triangle<Vector3<f32>>>,
  /// Material IDs for each triangle.
  pub materials: Vec<i32>,
  // TODO: Change this back to a hashmap once initial capacity is zero for those.
  /// Per-triangle bounding boxes.
  pub grass: Vec<Grass>,
  pub ids: Ids,
}

impl T {
  pub fn ids(&self) -> Ids {
    self.ids.clone()
  }

  pub fn is_empty(&self) -> bool {
    self.vertex_coordinates.is_empty()
  }
}

/// Construct an empty `T`.
pub fn empty(chunk_id: entity_id::T) -> T {
  T {
    vertex_coordinates: Vec::new(),
    normals: Vec::new(),

    ids: Ids::new(chunk_id),
    materials: Vec::new(),

    grass: Vec::new(),
  }
}
