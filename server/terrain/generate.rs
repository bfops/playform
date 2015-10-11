use cgmath::{Aabb3, Point3};
use std::sync::Mutex;
use stopwatch;

use voxel_data;
use isosurface_extraction;

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

impl isosurface_extraction::dual_contouring::material::T for voxel::Material {
  fn is_opaque(&self) -> bool {
    *self != voxel::Material::Empty
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

    isosurface_extraction::dual_contouring::run(
      voxels,
      mosaic,
      &voxel_data::bounds::new(
        position.as_pnt().x,
        position.as_pnt().y,
        position.as_pnt().z,
        terrain_block::LG_WIDTH,
      ),
      terrain_block::LG_SAMPLE_SIZE[lod_index.0 as usize] as i16,
      &mut |polygon| {
        let id = id_allocator.lock().unwrap().allocate();

        block.vertex_coordinates.push(tri(polygon.vertices[0], polygon.vertices[1], polygon.vertices[2]));
        block.normals.push(tri(polygon.normals[0], polygon.normals[1], polygon.normals[2]));
        block.materials.push(polygon.material as i32);
        block.ids.push(id);
        block.bounds.push((id, make_bounds(&polygon.vertices[0], &polygon.vertices[1], &polygon.vertices[2])));
      }
    );

    block
  })
}
