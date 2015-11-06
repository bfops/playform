use cgmath::{Aabb3, Point3, Point, Vector3};
use isosurface_extraction::dual_contouring;
use std::sync::Mutex;
use stopwatch;
use voxel_data;
use num::iter::range_inclusive;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::terrain_block;
use common::terrain_block::{TerrainBlock, tri};

use voxel;

#[inline]
fn partial_min<I, It>(mut it: It) -> Option<I>
where
  I: PartialOrd,
  It: Iterator<Item=I>,
{
  match it.next() {
    None => None,
    Some(mut best) => {
      for thing in it {
        if thing < best {
          best = thing;
        }
      }
      Some(best)
    },
  }
}

#[inline]
fn partial_max<I, It>(mut it: It) -> Option<I>
where
  I: PartialOrd,
  It: Iterator<Item=I>,
{
  match it.next() {
    None => None,
    Some(mut best) => {
      for thing in it {
        if thing > best {
          best = thing;
        }
      }
      Some(best)
    },
  }
}

fn make_bounds(
  v1: &Point3<f32>,
  v2: &Point3<f32>,
  v3: &Point3<f32>,
) -> Aabb3<f32> {
  let minx = *partial_min([v1.x, v2.x, v3.x].iter()).unwrap();
  let maxx = *partial_max([v1.x, v2.x, v3.x].iter()).unwrap();

  let miny = *partial_min([v1.y, v2.y, v3.y].iter()).unwrap();
  let maxy = *partial_max([v1.y, v2.y, v3.y].iter()).unwrap();

  let minz = *partial_min([v1.z, v2.z, v3.z].iter()).unwrap();
  let maxz = *partial_max([v1.z, v2.z, v3.z].iter()).unwrap();

  Aabb3::new(
    // TODO: Remove this - 1.0. It's a temporary hack until voxel collisions work,
    // to avoid zero-height Aabb3s.
    Point3::new(minx, miny - 1.0, minz),
    Point3::new(maxx, maxy, maxz),
  )
}

impl dual_contouring::material::T for voxel::Material {
  fn is_opaque(&self) -> bool {
    *self != voxel::Material::Empty
  }
}

mod voxel_storage {
  use voxel_data;
  use isosurface_extraction::dual_contouring;

  use voxel;

  pub struct T<'a, Mosaic> where Mosaic: 'a {
    pub voxels: &'a mut voxel::tree::T,
    pub mosaic: &'a Mosaic,
  }

  fn get_voxel<'a, Mosaic>(this: &mut T<'a, Mosaic>, bounds: &voxel_data::bounds::T) -> voxel::T<voxel::Material> where
    Mosaic: voxel_data::mosaic::T<voxel::Material>
  {
    let branch = this.voxels.get_mut_or_create(bounds);
    match branch {
      &mut voxel_data::tree::Empty => {
        let voxel = voxel_data::impls::surface_vertex::unwrap(voxel::of_field(this.mosaic, bounds));
        *branch =
          voxel_data::tree::Branch {
            data: Some(voxel.clone()),
            branches: Box::new(voxel_data::tree::Branches::empty()),
          };
        voxel
      },
      &mut voxel_data::tree::Branch { ref mut data, branches: _ }  => {
        match data {
          &mut None => {
            let voxel = voxel::unwrap(voxel::of_field(this.mosaic, bounds));
            *data = Some(voxel.clone());
            voxel
          },
          &mut Some(ref data) => {
            data.clone()
          },
        }
      },
    }
  }

  impl<'a, Mosaic> dual_contouring::voxel_storage::T<voxel::Material> for T<'a, Mosaic> where
    Mosaic: voxel_data::mosaic::T<voxel::Material> + 'a
  {
    fn get_material(&mut self, bounds: &voxel_data::bounds::T) -> voxel::Material {
      match get_voxel(self, bounds) {
        voxel::T::Surface(voxel) => voxel.corner,
        voxel::T::Volume(material) => material,
      }
    }

    fn get_voxel_data(&mut self, bounds: &voxel_data::bounds::T) -> dual_contouring::voxel_storage::VoxelData {
      match get_voxel(self, bounds) {
        voxel::T::Volume(_) => panic!("Can't extract voxel data from a volume"),
        voxel::T::Surface(voxel) => {
          dual_contouring::voxel_storage::VoxelData {
            bounds: bounds.clone(),
            vertex: voxel.surface_vertex.to_world_vertex(&bounds),
            normal: voxel.normal.to_float_normal(),
          }
        },
      }
    }
  }
}

/// Generate a `TerrainBlock` based on a given position in a `voxel::tree::T`.
/// Any necessary voxels will be generated.
pub fn generate_block<Mosaic>(
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  mosaic: &Mosaic,
  voxels: &mut voxel::tree::T,
  position: &BlockPosition,
  lod_index: LODIndex,
) -> TerrainBlock
  where Mosaic: voxel_data::mosaic::T<voxel::Material>,
{
  stopwatch::time("update.generate_block", || {
    let mut block = TerrainBlock::empty();

    let lg_edge_samples = terrain_block::LG_EDGE_SAMPLES[lod_index.0 as usize];
    let lg_sample_size = terrain_block::LG_SAMPLE_SIZE[lod_index.0 as usize];

    let low = position.as_pnt();
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

    {
      let mut edges = |direction, low_x, high_x, low_y, high_y, low_z, high_z| {
        for x in range_inclusive(low_x, high_x) {
        for y in range_inclusive(low_y, high_y) {
        for z in range_inclusive(low_z, high_z) {
          let edge = 
            dual_contouring::edge::T {
              low_corner: Point3::new(x, y, z),
              direction: direction,
              lg_size: lg_sample_size,
            };

          let mut voxel_storage =
            voxel_storage::T {
              voxels: voxels,
              mosaic: mosaic,
            };
          dual_contouring::edge::extract(
            &mut voxel_storage,
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

    block
  })
}
