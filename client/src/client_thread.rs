//! The client's "main" thread.

use client::LOD_THRESHOLDS;
use client_update::{ViewToClient, apply_view_to_client, apply_server_to_client};
use common::block_position::BlockPosition;
use common::communicate::{ClientToServer, ServerToClient};
use common::lod::LODIndex;
use common::process_events::process_channel;
use common::stopwatch::TimerSet;
use common::surroundings_loader::LODChange;
use std::num;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::time::duration::Duration;
use view_update::ClientToView;
use view_update::ClientToView::*;

#[allow(missing_docs)]
pub fn client_thread(
  my_url: String,
  ups_from_server: &Receiver<ServerToClient>,
  ups_to_server: &Sender<ClientToServer>,
  ups_from_view: &Receiver<ViewToClient>,
  ups_to_view: &Sender<ClientToView>,
) {
  let timers = TimerSet::new();
  let timers = &timers;

  ups_to_server.send(ClientToServer::Init(my_url)).unwrap();

  let mut client = None;

  loop {
    let quit =
      !process_channel(
        ups_from_view,
        |update| {
          apply_view_to_client(update, ups_to_server)
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

    client.as_mut().map(|client| {
      let block_position = BlockPosition::from_world_position(&client.player_position);

      let loaded_blocks = &mut client.loaded_blocks;
      client.surroundings_loader.update(
        block_position,
        |lod_change| {
          match lod_change {
            LODChange::Load(block_position, distance, _) => {
              timers.time("request_block", || {
                let lod = lod_index(distance);
                ups_to_server.send(ClientToServer::RequestBlock(block_position, lod)).unwrap();
              });
            },
            LODChange::Unload(block_position, _) => {
              loaded_blocks.remove(&block_position)
                // If it wasn't loaded, don't unload anything.
                .map(|(block, prev_lod)| {
                  timers.time("remove_block", || {
                    for id in block.ids.iter() {
                      ups_to_view.send(RemoveTerrain(*id)).unwrap();
                    }

                    ups_to_view.send(RemoveBlockData(block_position, prev_lod)).unwrap();
                  });
                });
            },
          };
        },
      );
    });

    timer::sleep(Duration::milliseconds(0));
  }

  timers.print();

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
