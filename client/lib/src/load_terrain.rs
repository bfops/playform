use cgmath::{Point3, Point};
use num;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use common::voxel;

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

  match voxels.entry(bounds) {
    Vacant(entry) => {
      entry.insert(*voxel);
    },
    Occupied(mut entry) => {
      if entry.insert(*voxel) == *voxel {
        return
      }
    },
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
pub fn load_edge<UpdateView>(
  client: &client::T,
  update_view: &mut UpdateView,
  edge: &edge::T,
) -> Result<(), ()> where
  UpdateView: FnMut(view_update::T),
{
  debug!("generate {:?}", edge);
  let voxels = client.voxels.lock().unwrap();
  let mut rng = client.rng.lock().unwrap();
  let mesh_fragment = try!(terrain_mesh::generate(&voxels, edge, &client.id_allocator, &mut *rng));

  let mut updates = Vec::new();

  let unload_fragments = client.loaded_edges.lock().unwrap().insert(&edge, mesh_fragment.clone());

  for mesh_fragment in unload_fragments {
    for id in &mesh_fragment.ids {
      updates.push(view_update::RemoveTerrain(*id));
    }
    for id in &mesh_fragment.grass_ids {
      updates.push(view_update::RemoveGrass(*id));
    }
  }

  if !mesh_fragment.ids.is_empty() {
    updates.push(view_update::AddBlock(mesh_fragment));
  }

  update_view(view_update::Atomic(updates));

  debug!("generate success!");
  Ok(())
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
