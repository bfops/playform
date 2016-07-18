use cgmath::{Point3, Point};
use num;

use common::voxel;

use chunk;
use client;
use edge;
use lod;
use terrain_mesh;
use view_update;

#[inline(never)]
pub fn load_voxel<Edge>(
  client: &client::T,
  voxel: &voxel::T,
  bounds: &voxel::bounds::T,
  mut touch_edge: Edge,
) where
  Edge: FnMut(edge::T),
{
  let mut voxels = client.voxels.lock().unwrap();

  {
    let voxel = Some(*voxel);
    let node = voxels.get_mut_or_create(bounds);
    let old_voxel = &mut node.data;
    if *old_voxel == voxel {
      return
    }
    *old_voxel = voxel;
  }

  trace!("voxel bounds {:?}", bounds);

  for direction in &[edge::Direction::X, edge::Direction::Y, edge::Direction::Z] {
    let p0 = Point3::new(bounds.x, bounds.y, bounds.z);
    let (v1, v2) = direction.perpendicular();
    let (v1, v2) = (v1.to_vec(), v2.to_vec());
    for p in &[p0, p0.add_v(&-direction.to_vec()), p0.add_v(&v1), p0.add_v(&v1).add_v(&v2), p0.add_v(&v2)] {
      touch_edge(
        edge::T {
          low_corner: *p,
          direction: *direction,
          lg_size: bounds.lg_size,
        }
      );
    }
  }
}

#[inline(never)]
pub fn load_chunk<TouchEdge>(
  client: &client::T,
  chunk: &chunk::T,
  mut touch_edge: TouchEdge,
) where
  TouchEdge: FnMut(edge::T),
{
  let mut voxels = client.voxels.lock().unwrap();

  for (bounds, voxel) in chunk.voxels() {
    voxels.get_mut_or_create(&bounds).data = Some(voxel);
  }

  for edge in chunk::edges(&chunk.position) {
    touch_edge(edge);
  }
}

pub fn lod_index(distance: i32) -> lod::T {
  assert!(distance >= 0);
  let mut lod = 0;
  while
    lod < client::LOD_THRESHOLDS.len()
    && client::LOD_THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  lod::T(num::traits::FromPrimitive::from_usize(lod).unwrap())
}
