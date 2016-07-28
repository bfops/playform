//! Data structure for a small chunk of terrain.

use cgmath;
use cgmath::{Array, Point3, Vector3, EuclideanSpace, InnerSpace, SquareMatrix, Rotation, Rotation3, ElementWise};
use collision::{Aabb, Aabb3};
use isosurface_extraction::dual_contouring;
use num::iter::range_inclusive;
use rand;
use std::f32;
use std::sync::{Arc, Mutex};
use stopwatch;

use common::entity_id;
use common::id_allocator;
use common::voxel;
// TODO: Move the server-only parts to the server, like BLOCK_WIDTH and sample_info.

use chunk_position;
use lod;

/// Number of LODs
pub const LOD_COUNT: usize = 4;
/// lg(WIDTH)
pub const LG_WIDTH: i16 = 3;
/// The width of a chunk of terrain.
pub const WIDTH: i32 = 1 << LG_WIDTH;

/// lg(EDGE_SAMPLES)
// NOTE: If there are duplicates here, weird invariants will fail.
// Just remove the LODs if you don't want duplicates.
pub const LG_EDGE_SAMPLES: [u16; LOD_COUNT] = [3, 2, 1, 0];
/// The number of voxels along an axis within a chunk, indexed by LOD.
pub const EDGE_SAMPLES: [u16; LOD_COUNT] = [
  1 << LG_EDGE_SAMPLES[0],
  1 << LG_EDGE_SAMPLES[1],
  1 << LG_EDGE_SAMPLES[2],
  1 << LG_EDGE_SAMPLES[3],
];

/// The width of a voxel within a chunk, indexed by LOD.
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

fn place_grass<T, Rng: rand::Rng>(
  polygon: &dual_contouring::polygon::T<T>,
  rng: &mut Rng,
) -> Vec<Grass> {
  let v = &polygon.vertices;
  let normal = (v[1] - v[0]).cross(v[2] - v[0]);
  let to_middle =
    &v[0].to_vec()
     + (&v[1].to_vec())
     + (&v[2].to_vec())
     / (3.0)
  ;

  let y = cgmath::Vector3::new(0.0, 1.0, 0.0);
  let z = cgmath::Vector3::new(0.0, 0.0, 1.0);

  let scale =
    (rng.next_f32() + 1.0) *
         (v[1] - v[0]).magnitude()
    .max((v[2] - v[1]).magnitude()
    .max((v[0] - v[2]).magnitude()));
  let scale = cgmath::Vector3::new(1.4, 0.4, 1.4).mul_element_wise(cgmath::Vector3::from_value(scale));

  let middle = Point3::from_vec(to_middle);
  let shift =
    (rng.next_f32() + 1.0) *
         (v[1] - middle).magnitude()
    .min((v[2] - middle).magnitude()
    .min((v[0] - middle).magnitude()));
  let shift_angle = rng.next_f32() * 2.0 * f32::consts::PI;
  let shift = cgmath::Matrix3::from_axis_angle(y, cgmath::rad(shift_angle)) * cgmath::Vector3::new(shift, 0.0, 0.0);

  let normal = normal.normalize();

  let rotate_normal_basis = cgmath::Basis3::between_vectors(y, normal);
  let rotate_normal: cgmath::Matrix3<f32> = From::from(rotate_normal_basis);

  // The direction the grass will point.
  // This is roughly straight up, perturbed randomly and squeezed into valid directions.
  let up = {
    let altitude = (rng.next_f32()  * 2.0 - 1.0) * f32::consts::PI / 8.0;
    let altitude: cgmath::Matrix3<f32> = cgmath::Matrix3::from_axis_angle(z, cgmath::rad(altitude));
    let azimuth = rng.next_f32() * 2.0 * f32::consts::PI;
    let azimuth: cgmath::Matrix3<f32> = cgmath::Matrix3::from_axis_angle(y, cgmath::rad(azimuth));
    let up = (rotate_normal * azimuth * altitude) * y;

    let axis = normal.cross(y).normalize();
    let dot = up.y;
    // Smoothly compress the y vector to be within 90 degrees of the normal.
    let c = (dot - 1.0).exp().acos();
    let rotate_normal = cgmath::Matrix3::from_axis_angle(axis, cgmath::rad(c));
    rotate_normal * up
  };

  // model space skew transformation to make the grass point in the right direction after rotation
  let skew = {
    // `up` in model space
    let up = rotate_normal_basis.invert().as_ref() * up;
    // dot with y
    let d = up.y;
    let skewness = (1.0 - d*d).sqrt();
    let mut skew = cgmath::Matrix4::from_value(1.0);
    skew[1][0] = up.x * skewness;
    skew[1][1] = d;
    skew[1][2] = up.z * skewness;
    skew
  };

  let rotate_normal = cgmath::Matrix4::from(rotate_normal);

  let rotate_model =
    cgmath::Quaternion::from_axis_angle(
      y,
      cgmath::rad(2.0 * f32::consts::PI * rng.next_f32()),
    );
  let rotate_model = cgmath::Matrix4::from(rotate_model);

  let translate = cgmath::Matrix4::from_translation(to_middle);

  let mut scale_mat = cgmath::Matrix4::from_value(1.0);
  scale_mat[0][0] = scale[0];
  scale_mat[1][1] = scale[1];
  scale_mat[2][2] = scale[2];

  let shift_mat = cgmath::Matrix4::from_translation(shift);

  let model = From::from(translate * rotate_normal * shift_mat * skew * rotate_model * scale_mat);

  let billboard_indices = [
    rng.gen_range(0, 9),
    rng.gen_range(0, 9),
    rng.gen_range(0, 9),
  ];
  vec!(
    Grass {
      model_matrix: model,
      normal: normal,
      tex_ids: billboard_indices,
    }
  )
}

pub fn generate<Rng: rand::Rng>(
  voxels: &voxel::tree::T,
  chunk_position: &chunk_position::T,
  lod: lod::T,
  id_allocator: &Mutex<id_allocator::T<entity_id::T>>,
  rng: &mut Rng,
) -> T
{
  stopwatch::time("terrain_mesh::generate", || {
    let mut chunk = empty();
    {
      let chunk2 = Arc::make_mut(&mut chunk);

      let lg_edge_samples = LG_EDGE_SAMPLES[lod.0 as usize];
      let lg_sample_size = LG_SAMPLE_SIZE[lod.0 as usize];

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
                  let id = id_allocator::allocate(id_allocator);

                  chunk2.vertex_coordinates.push(tri(polygon.vertices[0], polygon.vertices[1], polygon.vertices[2]));
                  chunk2.normals.push(tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]));
                  chunk2.materials.push(polygon.material as i32);
                  chunk2.ids.push(id);
                  let v = &polygon.vertices;
                  chunk2.bounds.push((id, make_bounds(&v[0], &v[1], &v[2])));

                  if polygon.material == voxel::Material::Terrain && lod <= lod::T(1) {
                    for grass in place_grass(&polygon, rng) {
                      let id = id_allocator::allocate(id_allocator);
                      chunk2.grass.push(grass);
                      chunk2.grass_ids.push(id);
                    }
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
  pub model_matrix: cgmath::Matrix4<f32>,
  pub normal: cgmath::Vector3<f32>,
  pub tex_ids: [u32; 3],
}

#[derive(Debug, Clone)]
/// A small continguous chunk of terrain.
pub struct T2 {
  // These Vecs must all be ordered the same way; each entry is the next triangle.

  /// Position of each vertex.
  pub vertex_coordinates: Vec<Triangle<Point3<f32>>>,
  /// Vertex normals. These should be normalized!
  pub normals: Vec<Triangle<Vector3<f32>>>,
  /// Entity IDs for each triangle.
  pub ids: Vec<entity_id::T>,
  /// Material IDs for each triangle.
  pub materials: Vec<i32>,
  // TODO: Change this back to a hashmap once initial capacity is zero for those.
  /// Per-triangle bounding boxes.
  pub bounds: Vec<(entity_id::T, Aabb3<f32>)>,

  pub grass: Vec<Grass>,
  pub grass_ids: Vec<entity_id::T>,
}

pub type T = Arc<T2>;

/// Construct an empty `T`.
pub fn empty() -> T {
  Arc::new(T2 {
    vertex_coordinates: Vec::new(),
    normals: Vec::new(),

    ids: Vec::new(),
    materials: Vec::new(),
    bounds: Vec::new(),

    grass: Vec::new(),
    grass_ids: Vec::new(),
  })
}
