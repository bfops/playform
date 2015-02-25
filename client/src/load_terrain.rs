use client::Client;
use common::block_position::BlockPosition;
use common::communicate::TerrainBlockSend;
use common::surroundings_loader::radius_between;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use update_surroundings::lod_index;
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

  match client.loaded_blocks.lock().unwrap().entry(block.position) {
    Vacant(entry) => {
      entry.insert((block.block.clone(), block.lod));
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
      entry.insert((block.block.clone(), block.lod));
    },
  };

  if !block.block.ids.is_empty() {
    update_view(ClientToView::AddBlock(block));
  }
}
