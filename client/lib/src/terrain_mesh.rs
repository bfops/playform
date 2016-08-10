//! Data structure for a small chunk of terrain.

use cgmath::{Point3, Vector3};
use collision::{Aabb, Aabb3};
use isosurface_extraction::dual_contouring;
use rand;
use std::f32;
use std::sync::{Arc, Mutex};

use common::entity_id;
use common::id_allocator;
use common::voxel;

use chunk;
use edge;
use lod;
use terrain;

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
  let min_x = f32::min(v1.x, f32::min(v2.x, v3.x));
  let max_x = f32::max(v1.x, f32::max(v2.x, v3.x));

  let min_y = f32::min(v1.y, f32::min(v2.y, v3.y));
  let max_y = f32::max(v1.y, f32::max(v2.y, v3.y));

  let min_z = f32::min(v1.z, f32::min(v2.z, v3.z));
  let max_z = f32::max(v1.z, f32::max(v2.z, v3.z));

  Aabb3::new(
    // TODO: Remove this - 1.0. It's a temporary hack until voxel collisions work,
    // to avoid zero-height Aabb3s.
    Point3::new(min_x, min_y - 1.0, min_z),
    Point3::new(max_x, max_y, max_z),
  )
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
  use cgmath;
  use isosurface_extraction::dual_contouring;

  use common::surroundings_loader;
  use common::voxel;

  use chunk;
  use lod;
  use terrain;

  pub struct T<'a> {
    pub chunks          : &'a terrain::Chunks,
    pub player_position : chunk::position::T,
  }

  fn get_voxel<'a>(this: &mut T<'a>, bounds: &voxel::bounds::T) -> Option<&'a voxel::T> {
    trace!("fetching {:?}", bounds);
    // find the LOD that this voxel _should_ be loaded at, and find it at that LOD.
    let chunk_position = chunk::position::containing(bounds);
    let distance =
      surroundings_loader::distance_between(
        &chunk_position.as_point,
        &this.player_position.as_point,
      );
    let corrected_lg_size = lod::of_distance(distance).lg_sample_size();
    this.chunks
      .get(&(chunk_position, corrected_lg_size))
      .map(|chunk| {
        // get the voxel position within the chunk
        // this is just a modulo by the chunk size.
        let mask = (1 << (chunk::LG_WIDTH as i32 - bounds.lg_size as i32)) - 1;
        let p =
          cgmath::Point3::new(
            bounds.x & mask,
            bounds.y & mask,
            bounds.z & mask,
          );
        chunk.get(&p)
      })
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

pub fn of_edges<Rng: rand::Rng, Edges: Iterator<Item=edge::T>>(
  chunks          : &terrain::Chunks,
  player_position : chunk::position::T,
  lod             : lod::T,
  id_allocator    : &Mutex<id_allocator::T<entity_id::T>>,
  rng             : &mut Rng,
  edges           : Edges,
) -> T
{
  let mut chunk = empty();
  {
    let chunk = Arc::make_mut(&mut chunk);
    for edge in edges {
      trace!("edge: {:?}", edge);
      let edge =
        dual_contouring::edge::T {
          low_corner : edge.low_corner,
          direction  :
            match edge.direction {
              edge::Direction::X => dual_contouring::edge::Direction::X,
              edge::Direction::Y => dual_contouring::edge::Direction::Y,
              edge::Direction::Z => dual_contouring::edge::Direction::Z,
            },
          lg_size    : edge.lg_size,
        };

      dual_contouring::edge::extract(
        &mut voxel_storage::T { chunks: chunks, player_position: player_position },
        &edge,
        &mut |polygon: dual_contouring::polygon::T<voxel::Material>| {
          let id = id_allocator::allocate(id_allocator);

          chunk.vertex_coordinates.push(tri(polygon.vertices[0], polygon.vertices[1], polygon.vertices[2]));
          chunk.normals.push(tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]));
          chunk.materials.push(polygon.material as i32);
          chunk.ids.terrain_ids.push(id);
          let v = &polygon.vertices;
          chunk.bounds.push((id, make_bounds(&v[0], &v[1], &v[2])));

          if polygon.material == voxel::Material::Terrain && lod <= lod::MAX_GRASS_LOD {
            chunk.grass.push(
              Grass {
                polygon_id: id,
                tex_id: rng.gen_range(0, 9),
              }
            );
            chunk.ids.grass_ids.push(id_allocator::allocate(id_allocator));
          }
        }
      ).unwrap();
    }
  }
  chunk
}

#[derive(Debug, Clone)]
pub struct Grass {
  pub polygon_id : entity_id::T,
  pub tex_id     : u32,
}

#[derive(Debug, Clone)]
pub struct Ids {
  /// Entity IDs for each triangle.
  pub terrain_ids: Vec<entity_id::T>,
  /// Entity IDs for each grass tuft.
  pub grass_ids: Vec<entity_id::T>,
}

impl Ids {
  pub fn new() -> Self {
    Ids {
      terrain_ids : Vec::new(),
      grass_ids   : Vec::new(),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.terrain_ids.is_empty() && self.grass_ids.is_empty()
  }
}

#[derive(Debug, Clone)]
/// A small continguous chunk of terrain.
pub struct T2 {
  // These Vecs must all be ordered the same way; each entry is the next triangle.

  /// Position of each vertex.
  pub vertex_coordinates: Vec<Triangle<Point3<f32>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<Triangle<Vector3<f32>>>,
  /// Material IDs for each triangle.
  pub materials: Vec<i32>,
  // TODO: Change this back to a hashmap once initial capacity is zero for those.
  /// Per-triangle bounding boxes.
  pub bounds: Vec<(entity_id::T, Aabb3<f32>)>,
  pub grass: Vec<Grass>,
  pub ids: Ids,
}

impl T2 {
  pub fn ids(&self) -> Ids {
    self.ids.clone()
  }
}

pub type T = Arc<T2>;

/// Construct an empty `T`.
pub fn empty() -> T {
  Arc::new(T2 {
    vertex_coordinates: Vec::new(),
    normals: Vec::new(),

    ids: Ids::new(),
    materials: Vec::new(),
    bounds: Vec::new(),

    grass: Vec::new(),
  })
}
