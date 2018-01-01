//! Data structure for a small chunk of terrain.

use cgmath::{Point3, Vector3};
use collision::{Aabb, Aabb3};
use isosurface_extraction::dual_contouring;
use num::iter::range_inclusive;
use rand;
use std::sync::Mutex;
use stopwatch;

use common::id_allocator;
use common::voxel;
// TODO: Move the server-only parts to the server, like BLOCK_WIDTH and sample_info.

use chunk;
use chunk_stats;
use lod;

use view;
use view::chunked_terrain;

#[derive(Debug, Copy, Clone)]
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

/// return a vector of all the voxels of a certain size that are contained within an AABB
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

#[allow(missing_docs)]
pub fn generate<Rng: rand::Rng>(
  voxels          : &voxel::tree::T,
  chunk_stats     : &mut chunk_stats::T,
  chunk_position  : &chunk::position::T,
  lod             : lod::T,
  chunk_allocator : &Mutex<id_allocator::T<view::entity::id::Terrain>>,
  grass_allocator : &Mutex<id_allocator::T<view::entity::id::Grass>>,
  rng             : &mut Rng,
) -> view::chunked_terrain::T
{
  stopwatch::time("terrain_mesh::generate", || {
    let lg_edge_samples = lod.lg_edge_samples();
    let lg_sample_size = lod.lg_sample_size();

    let mut chunked_terrain = chunked_terrain::empty();

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
                let vertices = tri(polygon.vertices[0], polygon.vertices[1], polygon.vertices[2]);
                let normals = tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]);
                let material = polygon.material as i32;

                let grass =
                  if polygon.material == voxel::Material::Terrain && lod <= lod::MAX_GRASS_LOD {
                    Some(chunked_terrain::PushGrass {
                      tex_id : rng.gen_range(0, 9),
                      id     : grass_allocator.lock().unwrap().allocate(),
                    })
                  } else {
                    None
                  };

                chunked_terrain.push(
                  &mut *chunk_allocator.lock().unwrap(),
                  vertices,
                  normals,
                  material,
                  grass,
                );
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

    chunk_stats.add(chunked_terrain.polygon_count());
    chunked_terrain
  })
}

/// All the information required to construct a grass tuft in vram
#[derive(Debug, Clone)]
pub struct Grass {
  /// subtexture indices
  pub tex_ids : Vec<u32>,
  #[allow(missing_docs)]
  pub ids : Vec<view::entity::id::Grass>,
  /// offset, relative to the beginning of the chunk, of the terrain polygon that a grass tuft rests on
  pub polygon_offsets : Vec<usize>,
}

impl Grass {
  #[allow(missing_docs)]
  pub fn new() -> Self {
    Grass {
      tex_ids         : Vec::new(),
      ids             : Vec::new(),
      polygon_offsets : Vec::new(),
    }
  }

  #[allow(missing_docs)]
  pub fn len(&self) -> usize {
    self.ids.len()
  }

  #[allow(missing_docs)]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct Ids {
  #[allow(missing_docs)]
  pub chunk_ids : Vec<view::entity::id::Terrain>,
  #[allow(missing_docs)]
  pub grass_ids : Vec<view::entity::id::Grass>,
}
