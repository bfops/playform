use client::Client;
use common::block_position::BlockPosition;
use common::communicate::TerrainBlockSend;
use common::surroundings_loader::radius_between;
use std::collections::hash_map::Entry::{Vacant, Occupied};
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::Mutex;
use std::time::duration::Duration;
use surroundings_thread::lod_index;
use view_update::ClientToView;

pub fn terrain_thread(
  client: &Client,
  to_load: &Receiver<TerrainBlockSend>,
  ups_to_view: &Mutex<Sender<ClientToView>>,
) {
  loop {
    // TODO: Is there a race where this block is stale by the time it gets to the client?
    let block = to_load.recv().unwrap();
    let player_position =
      BlockPosition::from_world_position(&client.player_position.lock().unwrap().clone());
    let distance = radius_between(&player_position, &block.position);
    if distance <= client.max_load_distance && lod_index(distance) == block.lod {
      match client.loaded_blocks.lock().unwrap().entry(block.position) {
        Vacant(entry) => {
          entry.insert((block.block.clone(), block.lod));
        },
        Occupied(mut entry) => {
          {
            // The block removal code is duplicated elsewhere.

            let &(ref prev_block, prev_lod) = entry.get();
            for &id in prev_block.ids.iter() {
              ups_to_view.lock().unwrap().send(ClientToView::RemoveTerrain(id)).unwrap();
            }
            ups_to_view.lock().unwrap().send(
              ClientToView::RemoveBlockData(block.position, prev_lod)
            ).unwrap();
          }
          entry.insert((block.block.clone(), block.lod));
        },
      };

      if !block.block.ids.is_empty() {
        ups_to_view.lock().unwrap().send(
          ClientToView::AddBlock(block),
        ).unwrap();
      }
    }

    timer::sleep(Duration::milliseconds(1));
  }
}
