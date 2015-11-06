use cgmath::{Aabb3, Point3};
use std::sync::Mutex;
use stopwatch;
use time;
use voxel_data;

use common::protocol;
use common::surroundings_loader;
use common::surroundings_loader::LoadType;
use common::voxel;

use block_position;
use client;
use load_terrain::{load_terrain_mesh, lod_index};
use server_update::apply_server_update;
use terrain_mesh;
use view_update::ClientToView;

const MAX_OUTSTANDING_TERRAIN_REQUESTS: u32 = 1 << 8;

pub fn update_thread<RecvServer, RecvBlock, UpdateView0, UpdateView1, UpdateServer, QueueBlock>(
  quit: &Mutex<bool>,
  client: &client::T,
  recv_server: &mut RecvServer,
  recv_block: &mut RecvBlock,
  update_view0: &mut UpdateView0,
  update_view1: &mut UpdateView1,
  update_server: &mut UpdateServer,
  queue_block: &mut QueueBlock,
) where
  RecvServer: FnMut() -> Option<protocol::ServerToClient>,
  RecvBlock: FnMut() -> Option<(voxel::T, voxel_data::bounds::T)>,
  UpdateView0: FnMut(ClientToView),
  UpdateView1: FnMut(ClientToView),
  UpdateServer: FnMut(protocol::ClientToServer),
  QueueBlock: FnMut(voxel::T, voxel_data::bounds::T),
{
  'update_loop: loop {
    if *quit.lock().unwrap() == true {
      break 'update_loop
    } else {
      stopwatch::time("update_iteration", || {
        let start = time::precise_time_ns();
        while let Some(up) = recv_server() {
          apply_server_update(
            client,
            update_view0,
            update_server,
            queue_block,
            up,
          );

          if time::precise_time_ns() - start >= 1_000_000 {
            break
          }
        }

        stopwatch::time("update_surroundings", || {
          let start = time::precise_time_ns();
          let player_position = *client.player_position.lock().unwrap();
          let player_position = block_position::of_world_position(&player_position);
          let mut loaded_blocks = client.loaded_blocks.lock().unwrap();
          let mut surroundings_loader = client.surroundings_loader.lock().unwrap();
          let mut updates = surroundings_loader.updates(player_position.as_pnt()) ;
          loop {
            if *client.outstanding_terrain_requests.lock().unwrap() >= MAX_OUTSTANDING_TERRAIN_REQUESTS {
              debug!("update loop breaking");
              break;
            }

            let block_position;
            let load_type;
            match updates.next() {
              None => break,
              Some((b, l)) => {
                block_position = block_position::of_pnt(&b);
                load_type = l;
              },
            }

            debug!("block surroundings");
            let distance =
              surroundings_loader::distance_between(
                player_position.as_pnt(),
                block_position.as_pnt(),
              );
            match load_type {
              LoadType::Load => {
                stopwatch::time("update_thread.load_block", || {
                  let lod = lod_index(distance);
                  let loaded_lod =
                    loaded_blocks
                    .get(&block_position)
                    .map(|&(_, lod)| lod);
                  if loaded_lod != Some(lod) {
                    let voxel_size = 1 << terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize];
                    update_server(
                      protocol::ClientToServer::RequestVoxels(
                        client.id,
                        terrain_mesh::voxels_in(
                          &Aabb3::new(
                            Point3::new(
                              (block_position.as_pnt().x << terrain_mesh::LG_WIDTH) - voxel_size,
                              (block_position.as_pnt().y << terrain_mesh::LG_WIDTH) - voxel_size,
                              (block_position.as_pnt().z << terrain_mesh::LG_WIDTH) - voxel_size,
                            ),
                            Point3::new(
                              ((block_position.as_pnt().x + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                              ((block_position.as_pnt().y + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                              ((block_position.as_pnt().z + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                            ),
                          ),
                          terrain_mesh::LG_SAMPLE_SIZE[lod.0 as usize],
                        ),
                      )
                    );
                    debug!("{:?} Sending a block {:?}", player_position, block_position);
                    *client.outstanding_terrain_requests.lock().unwrap() += 1;
                  } else {
                    debug!("Not re-loading {:?} at {:?}", block_position, lod);
                  }
                })
              },
              LoadType::Update => {
                stopwatch::time("update_thread.update_block", || {
                  let new_lod = lod_index(distance);
                  let lod_change =
                    loaded_blocks
                    .get(&block_position)
                    .map(|&(_, lod)| new_lod < lod);
                  if lod_change == Some(true) {
                    debug!("Sending a block");
                    let voxel_size = 1 << terrain_mesh::LG_SAMPLE_SIZE[new_lod.0 as usize];
                    update_server(
                      protocol::ClientToServer::RequestVoxels(
                        client.id,
                        terrain_mesh::voxels_in(
                          &Aabb3::new(
                            Point3::new(
                              (block_position.as_pnt().x << terrain_mesh::LG_WIDTH) - voxel_size,
                              (block_position.as_pnt().y << terrain_mesh::LG_WIDTH) - voxel_size,
                              (block_position.as_pnt().z << terrain_mesh::LG_WIDTH) - voxel_size,
                            ),
                            Point3::new(
                              ((block_position.as_pnt().x + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                              ((block_position.as_pnt().y + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                              ((block_position.as_pnt().z + 1) << terrain_mesh::LG_WIDTH) + voxel_size,
                            ),
                          ),
                          terrain_mesh::LG_SAMPLE_SIZE[new_lod.0 as usize],
                        ),
                      )
                    );
                    *client.outstanding_terrain_requests.lock().unwrap() += 1;
                  } else {
                    trace!("Not updating {:?} at {:?}", block_position, new_lod);
                  }
                })
              },
              LoadType::Unload => {
                stopwatch::time("update_thread.unload", || {
                  // The block removal code is duplicated elsewhere.

                  loaded_blocks
                    .remove(&block_position)
                    // If it wasn't loaded, don't unload anything.
                    .map(|(block, _)| {
                      for id in &block.ids {
                        update_view1(ClientToView::RemoveTerrain(*id));
                      }
                    });
                })
              },
            };

            if time::precise_time_ns() - start >= 1_000_000 {
              break
            }
          }
        });

        let start = time::precise_time_ns();
        while let Some((block, bounds)) = recv_block() {
          trace!("Got block: {:?} at {:?}", block, bounds);
          load_terrain_mesh(
            client,
            update_view1,
            block,
            bounds,
          );

          if time::precise_time_ns() - start >= 1_000_000 {
            break
          }
        }
      })
    }
  }
}
