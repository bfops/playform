//! Data structure for a small block of terrain.

use cgmath;
use cgmath::{Point, Point3, Vector3, Vector, EuclideanVector, Matrix, Rotation, Aabb, Aabb3};
use isosurface_extraction::dual_contouring;
use rand;
use rand::Rng;
use std::f32;
use std::sync::{Arc, Mutex};
use stopwatch;

use common::entity_id;
use common::id_allocator;
use common::voxel;
// TODO: Move the server-only parts to the server, like BLOCK_WIDTH and sample_info.

use edge;

// TODO: terrain_mesh is now chunk-agnostic. Some/all of these values should be moved.
/// Number of LODs
pub const LOD_COUNT: usize = 4;
/// lg(WIDTH)
pub const LG_WIDTH: i16 = 3;
/// The width of a block of terrain.
pub const WIDTH: i32 = 1 << LG_WIDTH;

/// lg(EDGE_SAMPLES)
// NOTE: If there are duplicates here, weird invariants will fail.
// Just remove the LODs if you don't want duplicates.
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

mod voxel_storage {
  use isosurface_extraction::dual_contouring;

  use common::voxel;

  pub struct T<'a> {
    pub voxels: &'a voxel::tree::T,
  }

  fn get_voxel<'a>(this: &mut T<'a>, bounds: &voxel::bounds::T) -> Option<&'a voxel::T> {
    trace!("fetching {:?}", bounds);
    this.voxels.get(bounds)
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
  let normal = v[1].sub_p(&v[0]).cross(&v[2].sub_p(&v[0]));
  let to_middle =
    &v[0].to_vec()
    .add_v(&v[1].to_vec())
    .add_v(&v[2].to_vec())
    .div_s(3.0)
  ;

  let y = cgmath::Vector3::new(0.0, 1.0, 0.0);
  let z = cgmath::Vector3::new(0.0, 0.0, 1.0);

  let scale =
    (rng.next_f32() + 1.0) *
         (v[1].sub_p(&v[0]).length())
    .max((v[2].sub_p(&v[1]).length())
    .max((v[0].sub_p(&v[2]).length())));
  let scale = cgmath::Vector3::new(1.4, 0.4, 1.4) * cgmath::Vector3::from_value(scale);

  let middle = Point::from_vec(to_middle);
  let shift =
    (rng.next_f32() + 1.0) *
         (v[1].sub_p(&middle).length())
    .min((v[2].sub_p(&middle).length())
    .min((v[0].sub_p(&middle).length())));
  let shift_angle = rng.next_f32() * 2.0 * f32::consts::PI;
  let shift = cgmath::Matrix3::from_axis_angle(&y, cgmath::rad(shift_angle)).mul_v(&cgmath::Vector3::new(shift, 0.0, 0.0));

  let normal = normal.normalize();

  let rotate_normal_basis: cgmath::Basis3<f32> = cgmath::Rotation::between_vectors(&y, &normal);
  let rotate_normal: cgmath::Matrix3<f32> = From::from(rotate_normal_basis);

  // The direction the grass will point.
  // This is roughly straight up, perturbed randomly and squeezed into valid directions.
  let up = {
    let altitude = (rng.next_f32()  * 2.0 - 1.0) * f32::consts::PI / 8.0;
    let altitude: cgmath::Matrix3<f32> = cgmath::Matrix3::from_axis_angle(&z, cgmath::rad(altitude));
    let azimuth = rng.next_f32() * 2.0 * f32::consts::PI;
    let azimuth: cgmath::Matrix3<f32> = cgmath::Matrix3::from_axis_angle(&y, cgmath::rad(azimuth));
    let up = (rotate_normal * azimuth * altitude).mul_v(&y);

    let axis = normal.cross(&y).normalize();
    let dot = up.y;
    // Smoothly compress the y vector to be within 90 degrees of the normal.
    let c = (dot - 1.0).exp().acos();
    let rotate_normal = cgmath::Matrix3::from_axis_angle(&axis, cgmath::rad(c));
    rotate_normal.mul_v(&up)
  };

  // model space skew transformation to make the grass point in the right direction after rotation
  let skew = {
    // `up` in model space
    let up = rotate_normal_basis.invert().as_ref().mul_v(&up);
    // dot with y
    let d = up.y;
    let skewness = (1.0 - d*d).sqrt();
    let mut skew = cgmath::Matrix4::from_value(1.0);
    skew[1][0] = up.x * skewness;
    skew[1][1] = d;
    skew[1][2] = up.z * skewness;
    skew
  };

  let rotate_normal: cgmath::Matrix4<f32> = From::from(rotate_normal);

  let rotate_model: cgmath::Quaternion<f32> = cgmath::Rotation3::from_axis_angle(&y, cgmath::rad(2.0 * f32::consts::PI * rng.next_f32()));
  let rotate_model: cgmath::Matrix4<f32> = From::from(rotate_model);

  let translate = cgmath::Matrix4::from_translation(&to_middle);

  let mut scale_mat = cgmath::Matrix4::from_value(1.0);
  scale_mat[0][0] = scale[0];
  scale_mat[1][1] = scale[1];
  scale_mat[2][2] = scale[2];

  let shift_mat = cgmath::Matrix4::from_translation(&shift);

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
  edge: &edge::T,
  id_allocator: &Mutex<id_allocator::T<entity_id::T>>,
  rng: &mut Rng,
) -> Result<T, ()>
{
  stopwatch::time("terrain_mesh::generate", || {
    let mut block = empty();
    {
      let block2 = Arc::make_mut(&mut block);

      let low = edge.low_corner;
      let high = low.add_v(&Vector3::new(1, 1, 1));
      let low =
        Point3::new(
          low.x << edge.lg_size,
          low.y << edge.lg_size,
          low.z << edge.lg_size,
        );
      let high =
        Point3::new(
          high.x << edge.lg_size,
          high.y << edge.lg_size,
          high.z << edge.lg_size,
        );

      trace!("low {:?}", low);
      trace!("high {:?}", high);

      trace!("edge: {:?} {:?}", edge.direction, low);

      try!(dual_contouring::edge::extract(
        &mut voxel_storage::T { voxels: voxels },
        &dual_contouring::edge::T {
          low_corner: edge.low_corner,
          lg_size: edge.lg_size,
          direction:
            match edge.direction {
              edge::Direction::X => dual_contouring::edge::Direction::X,
              edge::Direction::Y => dual_contouring::edge::Direction::Y,
              edge::Direction::Z => dual_contouring::edge::Direction::Z,
            },
        },
        &mut |polygon: dual_contouring::polygon::T<voxel::Material>| {
          let id = id_allocator::allocate(id_allocator);

          let v = &polygon.vertices;
          block2.vertex_coordinates.push(tri(v[0], v[1], v[2]));
          block2.normals.push(tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]));
          block2.materials.push(polygon.material as i32);
          block2.ids.push(id);
          block2.bounds.push((id, make_bounds(&v[0], &v[1], &v[2])));

          if polygon.material == voxel::Material::Terrain && edge.lg_size <= LG_SAMPLE_SIZE[1] {
            for grass in place_grass(&polygon, rng) {
              let id = id_allocator::allocate(id_allocator);
              block2.grass.push(grass);
              block2.grass_ids.push(id);
            }
          }
        }
      ));
    }
    Ok(block)
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
