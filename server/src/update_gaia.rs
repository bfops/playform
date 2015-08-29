/// Creator of the earth.

use cgmath::{Aabb3, Point, Vector, Point3, Vector3};
use std::ops::DerefMut;
use stopwatch;

use common::communicate::{ClientId, ServerToClient, TerrainBlockSend};
use common::lod::{LODIndex, OwnerId};
use common::serialize::Copyable;
use common::block_position::BlockPosition;

use server::Server;
use terrain;
use terrain_loader::TerrainLoader;
use voxel;

#[derive(Debug, Clone, Copy)]
pub enum LoadReason {
  Local(OwnerId),
  ForClient(ClientId),
}

#[derive(Debug)]
pub enum ServerToGaia {
  Load(BlockPosition, LODIndex, LoadReason),
  Brush(voxel::brush::Action, terrain::voxel::brush::sphere::T),
}

// TODO: Consider adding terrain loads to a thread pool instead of having one monolithic separate thread.
pub fn update_gaia(
  server: &Server,
  update: ServerToGaia,
) {
  match update {
    ServerToGaia::Load(position, lod, load_reason) => {
      stopwatch::time("terrain.load", || {
        // TODO: Just lock `terrain` for the check and then the move;
        // don't lock for the whole time where we're generating the block.
        let mut terrain_loader = server.terrain_loader.lock().unwrap();
        let terrain_loader = terrain_loader.deref_mut();
        let lod_map = &mut terrain_loader.lod_map;
        let in_progress_terrain = &mut terrain_loader.in_progress_terrain;
        let block =
          terrain_loader.terrain.load(
            &server.id_allocator,
            &position,
            lod,
          );

        match load_reason {
          LoadReason::Local(owner) => {
            // TODO: Check that this block isn't stale, i.e. should still be loaded.
            // Maybe this should just ping the original thread, same as we ping the client.
            TerrainLoader::insert_block(
              block,
              &position,
              lod,
              owner,
              &server.physics,
              lod_map,
              in_progress_terrain,
            );
          },
          LoadReason::ForClient(id) => {
            let clients = server.clients.lock().unwrap();
            let client = clients.get(&id).unwrap();
            client.sender.send(Some(
              ServerToClient::UpdateBlock(TerrainBlockSend {
                position: Copyable(position),
                block: block.clone(),
                lod: Copyable(lod),
              })
            )).unwrap();
          },
        }
      });
    },
    ServerToGaia::Brush(action, brush) => {
      let mut terrain_loader = server.terrain_loader.lock().unwrap();
      let id_allocator = &server.id_allocator;
      let r = brush.radius + 1.0;
      let brush_bounds =
        Aabb3::new(
          {
            let low = brush.center.add_v(&-Vector3::new(r, r, r));
            Point3::new(low.x.floor() as i32, low.y.floor() as i32, low.z.floor() as i32)
          },
          {
            let high = brush.center.add_v(&Vector3::new(r, r, r));
            Point3::new(high.x.ceil() as i32, high.y.ceil() as i32, high.z.ceil() as i32)
          },
        );
      debug!("remove {:?} with bounds {:?}", brush, brush_bounds);
      terrain_loader.terrain.brush(
        id_allocator,
        &brush,
        &brush_bounds,
        action,
        |block, position, lod| {
          let clients = server.clients.lock().unwrap();
          for client in clients.values() {
            client.sender.send(Some(
              ServerToClient::UpdateBlock(TerrainBlockSend {
                position: Copyable(*position),
                block: block.clone(),
                lod: Copyable(lod),
              })
            )).unwrap();
          }
        },
      );
    }
  };
}
