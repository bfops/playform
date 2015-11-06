use num;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use voxel_data;

use common::surroundings_loader;
use common::voxel;

use block_position;
use client;
use lod;
use terrain_block;
use view_update::ClientToView;

pub fn load_terrain_block<UpdateView>(
  client: &client::T,
  update_view: &mut UpdateView,
  voxel: voxel::T,
  bounds: voxel_data::bounds::T,
) where
  UpdateView: FnMut(ClientToView),
{
  let player_position =
    block_position::of_world_position(&client.player_position.lock().unwrap().clone());
  let block_position = block_position::containing_voxel(&bounds);

  let distance = surroundings_loader::distance_between(player_position.as_pnt(), &block_position.as_pnt());
  let lod = lod_index(distance);

  // Make sure the block for this voxel is at the right LOD.
  {
    let mut partial_blocks = client.partial_blocks.lock().unwrap();
    let partial_block =
      partial_blocks.entry(block_position)
      .or_insert_with(|| terrain_block::Partial::new(block_position, lod));
    if lod != partial_block.lod() {
      *partial_block = terrain_block::Partial::new(block_position, lod);
    }
  }

  for dx in num::range_inclusive(-1, 1) {
  for dy in num::range_inclusive(-1, 1) {
  for dz in num::range_inclusive(-1, 1) {
    let block_position =
      block_position::containing_voxel(
        &voxel_data::bounds::new(
          bounds.x + dx,
          bounds.y + dy,
          bounds.z + dz,
          bounds.lg_size,
        )
      );

    let distance = surroundings_loader::distance_between(player_position.as_pnt(), &block_position.as_pnt());

    if distance > client.max_load_distance {
      debug!(
        "Not loading {:?}: too far away from player at {:?}.",
        bounds,
        player_position,
      );
      continue;
    }

    let lod = lod_index(distance);
    let lg_size = terrain_block::LG_SAMPLE_SIZE[lod.0 as usize];
    if lg_size != bounds.lg_size {
      debug!(
        "Not loading {:?} is not the desired LOD {:?}.",
        bounds,
        lod
      );
      continue;
    }

    let mut partial_blocks = client.partial_blocks.lock().unwrap();
    {
      let partial_block =
        partial_blocks.entry(block_position)
        .or_insert_with(|| terrain_block::Partial::new(block_position, lod));
      if lod != partial_block.lod() {
        continue;
      }

      partial_block.add(voxel, bounds);
      trace!("Adding {:?} to {:?}", bounds, block_position);

      match partial_block.finalize(&client.id_allocator) {
        None => continue,
        Some(mesh_block) => load_mesh_block(client, update_view, &block_position, lod, mesh_block),
      };
    }
    partial_blocks.remove(&block_position).unwrap();
  }}}
}

pub fn load_mesh_block<UpdateView>(
  client: &client::T,
  update_view: &mut UpdateView,
  block_position: &block_position::T,
  lod: lod::T,
  mesh_block: terrain_block::T,
) where
  UpdateView: FnMut(ClientToView),
{
  let mut updates = Vec::new();

  // TODO: Rc instead of clone.
  match client.loaded_blocks.lock().unwrap().entry(block_position.clone()) {
    Vacant(entry) => {
      entry.insert((mesh_block.clone(), lod));
    },
    Occupied(mut entry) => {
      {
        // The mesh_block removal code is duplicated elsewhere.

        let &(ref prev_block, _) = entry.get();
        for &id in &prev_block.ids {
          updates.push(ClientToView::RemoveTerrain(id));
        }
      }
      entry.insert((mesh_block.clone(), lod));
    },
  };

  if !mesh_block.ids.is_empty() {
    updates.push(ClientToView::AddBlock(block_position.clone(), mesh_block, lod));
  }

  update_view(ClientToView::Atomic(updates));
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
