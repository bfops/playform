//! The client's "main" thread.

use client::{Client, LOD_THRESHOLDS};
use client_update::{ViewToClient, apply_view_to_client, apply_server_to_client};
use common::block_position::BlockPosition;
use common::communicate::{ClientToServer, ServerToClient};
use common::lod::{LODIndex, OwnerId};
use common::process_events::process_channel;
use common::surroundings_loader::LODChange;
use std::num;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::time::duration::Duration;
use view_update::ClientToView;
use view_update::ClientToView::*;

#[allow(missing_docs)]
pub fn client_thread<'a, 'b>(
  client_id: OwnerId,
  ups_from_server: &'a Receiver<ServerToClient>,
  ups_to_server: &'a Sender<ClientToServer>,
  ups_from_view: &'b Receiver<ViewToClient>,
  ups_to_view: &'b Sender<ClientToView>,
) {
  let mut client = Client::new(client_id);

  loop {
    let quit =
      !process_channel(
        ups_from_view,
        |update| {
          apply_view_to_client(update, ups_to_server);
          true
        },
      );
    if quit {
      break;
    }

    process_channel(
      ups_from_server,
      |update| {
        apply_server_to_client(update, &mut client, &ups_to_view);
        true
      },
    );

    let block_position = BlockPosition::from_world_position(&client.player_position);

    let loaded_blocks = &mut client.loaded_blocks;
    client.surroundings_loader.update(
      block_position,
      |lod_change| {
        match lod_change {
          LODChange::Load(block_position, distance, _) => {
            let lod = lod_index(distance);
            ups_to_server.send(ClientToServer::RequestBlock(block_position, lod)).unwrap();
          },
          LODChange::Unload(block_position, _) => {
            loaded_blocks.remove(&block_position)
              // If it wasn't loaded, don't unload anything.
              .map(|(block, prev_lod)| {
                for id in block.ids.iter() {
                  ups_to_view.send(RemoveTerrain(*id)).unwrap();
                }

                ups_to_view.send(RemoveBlockData(block_position, prev_lod)).unwrap();
              });
          },
        };
      },
    );

    timer::sleep(Duration::milliseconds(0));
  }

  debug!("client exiting.");
}

fn lod_index(distance: i32) -> LODIndex {
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
