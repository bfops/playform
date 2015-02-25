use common::block_position::BlockPosition;
use common::communicate::ClientToServer;
use common::surroundings_loader::SurroundingsLoader;

use client::{Client, LOD_THRESHOLDS};
use update_surroundings;
use view_update::ClientToView;

pub fn surroundings_thread<Recv, UpdateView, UpdateServer>(
  client: &Client,
  recv: &mut Recv,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
) where
  Recv: FnMut() -> Option<BlockPosition>,
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
{
  let max_load_distance = client.max_load_distance;

  let mut surroundings_loader = {
    SurroundingsLoader::new(
      max_load_distance,
      LOD_THRESHOLDS.iter().map(|&x| x).collect(),
    )
  };

  while let Some(position) = recv() {
    update_surroundings::update_surroundings(
      client,
      update_view,
      update_server,
      &mut surroundings_loader,
      &position,
    )
  }
}
