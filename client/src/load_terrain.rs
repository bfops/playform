use num;
use std::collections::hash_map::Entry::{Vacant, Occupied};

use common::block_position;
use common::block_position::BlockPosition;
use common::communicate::TerrainBlockSend;
use common::lod::LODIndex;

use client::{Client, LOD_THRESHOLDS};
use view_update::ClientToView;

pub fn load_terrain_block<UpdateView>(
  client: &Client,
  update_view: &mut UpdateView,
  block: TerrainBlockSend,
) where
  UpdateView: FnMut(ClientToView),
{
  let player_position =
    BlockPosition::from_world_position(&client.player_position.lock().unwrap().clone());
  let distance = block_position::distance(&player_position, &block.position.0);

  if distance > client.max_load_distance {
    debug!(
      "Not loading {:?}: too far away from player at {:?}.",
      block.position,
      player_position,
    );
    return;
  }

  let lod = lod_index(distance);
  if lod != block.lod.0 {
    debug!(
      "Not loading {:?}: given LOD {:?} is not the desired LOD {:?}.",
      block.position,
      block.lod,
      lod
    );
    return;
  }

  let mut updates = Vec::new();

  match client.loaded_blocks.lock().unwrap().entry(block.position.0) {
    Vacant(entry) => {
      entry.insert((block.block.clone(), block.lod.0));
    },
    Occupied(mut entry) => {
      {
        // The block removal code is duplicated elsewhere.

        let &(ref prev_block, prev_lod) = entry.get();
        for &id in &prev_block.ids {
          updates.push(ClientToView::RemoveTerrain(id));
        }
        updates.push(ClientToView::RemoveBlockData(block.position.0, prev_lod));
      }
      entry.insert((block.block.clone(), block.lod.0));
    },
  };

  if !block.block.ids.is_empty() {
    updates.push(ClientToView::AddBlock(block.position.0, block.block, block.lod.0));
  }

  update_view(ClientToView::Atomic(updates));
}

pub fn lod_index(distance: i32) -> LODIndex {
  assert!(distance >= 0);
  let mut lod = 0;
  while
    lod < LOD_THRESHOLDS.len()
    && LOD_THRESHOLDS[lod] < distance
  {
    lod += 1;
  }
  LODIndex(num::traits::FromPrimitive::from_usize(lod).unwrap())
}
