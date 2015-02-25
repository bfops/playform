use std::num;

use common::block_position::BlockPosition;
use common::communicate::ClientToServer;
use common::lod::LODIndex;
use common::surroundings_loader::{LODChange, SurroundingsLoader};

use client::{Client, LOD_THRESHOLDS};
use view_update::ClientToView;
use view_update::ClientToView::*;

pub fn update_surroundings<UpdateView, UpdateServer>(
  client: &Client,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
  surroundings_loader: &mut SurroundingsLoader,
  position: &BlockPosition,
) where
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
{
  surroundings_loader.update(
    *position,
    |lod_change| {
      match lod_change {
        LODChange::Load(block_position, distance) => {
          let lod = lod_index(distance);
          update_server(ClientToServer::RequestBlock(block_position, lod));
        },
        LODChange::Unload(block_position) => {
          // The block removal code is duplicated elsewhere.

          client.loaded_blocks
            .lock().unwrap()
            .remove(&block_position)
            // If it wasn't loaded, don't unload anything.
            .map(|(block, prev_lod)| {
              for id in block.ids.iter() {
                update_view(RemoveTerrain(*id));
              }

              update_view(RemoveBlockData(block_position, prev_lod));
            });
        },
      };
    },
  );
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
