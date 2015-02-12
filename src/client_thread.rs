use client::Client;
use client_update::{ServerToClient, ViewToClient};
use lod_map::{LOD, OwnerId};
use server_update::ClientToServer;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::time::duration::Duration;
use surroundings_loader::LODChange;
use terrain::terrain_block::BlockPosition;
use view_update::ClientToView;
use view_update::ClientToView::*;

pub fn client_thread(
  client_id: OwnerId,
  ups_from_server: &Receiver<ServerToClient>,
  ups_to_server: &Sender<ClientToServer>,
  ups_from_view: &Receiver<ViewToClient>,
  ups_to_view: &Sender<ClientToView>,
) {
  let mut client = Client::new(client_id);

  'client_loop:loop {
    match ups_from_view.try_recv() {
      Err(TryRecvError::Empty) => {},
      Err(e) => panic!("Error getting client local updates: {:?}", e),
      Ok(update) => {
        if !update.apply(&ups_to_server) {
          break 'client_loop;
        }
      },
    };

    'event_loop:loop {
      let update;
      match ups_from_server.try_recv() {
        Err(TryRecvError::Empty) => break 'event_loop,
        Err(e) => panic!("Error getting client local updates: {:?}", e),
        Ok(e) => update = e,
      };
      update.apply(&mut client, &ups_to_view);
    }

    let block_position = BlockPosition::from_world_position(&client.player_position);

    let loaded_blocks = &mut client.loaded_blocks;
    client.surroundings_loader.update(
      block_position,
      |lod_change| {
        match lod_change {
          LODChange::Load(block_position, lod, _) => {
            if let LOD::LodIndex(lod) = lod {
              ups_to_server.send(ClientToServer::RequestBlock(block_position, lod)).unwrap();
            } else {
              panic!("Clients should not load placeholders.");
            }
          },
          LODChange::Unload(block_position, _) => {
            // If it wasn't loaded, don't unload anything.
            loaded_blocks.remove(&block_position).map(|(block, prev_lod)| {
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
