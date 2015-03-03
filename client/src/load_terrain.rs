use rustc_serialize::json;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use std::num;

use common::block_position::BlockPosition;
use common::communicate::TerrainBlockSend;
use common::lod::LODIndex;
use common::surroundings_loader::radius_between;
use common::terrain_block::TerrainBlock;

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
  let distance = radius_between(&player_position, &block.position);

  if distance > client.max_load_distance {
    debug!(
      "Not loading {:?}: too far away from player at {:?}.",
      block.position,
      player_position,
    );
    return;
  }

  let lod = lod_index(distance);
  if lod != block.lod {
    debug!(
      "Not loading {:?}: given LOD {:?} is not the desired LOD {:?}.",
      block.position,
      block.lod,
      lod
    );
    return;
  }

  // TODO: This block is decoded both here and in update_view.
  // Fix this.
  let actual_block: TerrainBlock = json::decode(&block.block).unwrap();
  match client.loaded_blocks.lock().unwrap().entry(block.position) {
    Vacant(entry) => {
      entry.insert((actual_block.clone(), block.lod));
    },
    Occupied(mut entry) => {
      {
        // The block removal code is duplicated elsewhere.

        let &(ref prev_block, prev_lod) = entry.get();
        for &id in prev_block.ids.iter() {
          update_view(ClientToView::RemoveTerrain(id));
        }
        update_view(ClientToView::RemoveBlockData(block.position, prev_lod));
      }
      entry.insert((actual_block.clone(), block.lod));
    },
  };

  if !actual_block.ids.is_empty() {
    update_view(ClientToView::AddBlock((block.position, actual_block, block.lod)));
  }
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
  LODIndex(num::cast(lod).unwrap())
}
