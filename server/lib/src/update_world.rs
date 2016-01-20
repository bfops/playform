use cgmath::{Point, Point3, Vector, Vector3};
use std::ops::Neg;
use stopwatch;

use common::protocol;
use common::surroundings_loader::LoadType;
use common::voxel;

use lod;
use mob;
use server::Server;
use update_gaia;

// TODO: Consider removing the IntervalTimer.

pub fn update_world<RequestBlock>(
  server: &Server,
  request_block: &mut RequestBlock,
) where
  RequestBlock: FnMut(update_gaia::Message),
{
  stopwatch::time("update_world", || {
    stopwatch::time("update_world.player", || {
      for (_, player) in server.players.lock().unwrap().iter_mut() {
        player.update(server, request_block);
      }

      let players: Vec<_> = server.players.lock().unwrap().keys().cloned().collect();
      for (_, client) in server.clients.lock().unwrap().iter_mut() {
        for &id in &players {
          let bounds = server.physics.lock().unwrap().get_bounds(id).unwrap().clone();
          client.send(protocol::ServerToClient::UpdatePlayer(id, bounds));
        }
      }
    });

    stopwatch::time("update_world.mobs", || {
      for (_, mob) in server.mobs.lock().unwrap().iter_mut() {
        let position =
          Point3::new(
            mob.position.x as i32,
            mob.position.y as i32,
            mob.position.z as i32,
          );

        let owner_id = mob.owner_id;
        for (position, load_type) in mob.surroundings_loader.updates(&position) {
          load_placeholders(
            owner_id,
            server,
            request_block,
            &voxel::bounds::new(position.x, position.y, position.z, 0),
            load_type,
          )
        }

        {
          let behavior = mob.behavior;
          (behavior)(server, mob);
        }

        mob.speed = mob.speed.add_v(&-Vector3::new(0.0, 0.1, 0.0 as f32));

        // TODO: This logic is dumb (isolating along components shouldn't be a thing). Change it.
        let delta_p = mob.speed;
        if delta_p.x != 0.0 {
          translate_mob(server, mob, &Vector3::new(delta_p.x, 0.0, 0.0));
        }
        if delta_p.y != 0.0 {
          translate_mob(server, mob, &Vector3::new(0.0, delta_p.y, 0.0));
        }
        if delta_p.z != 0.0 {
          translate_mob(server, mob, &Vector3::new(0.0, 0.0, delta_p.z));
        }
      }
    });

    server.sun.lock().unwrap().update().map(|fraction| {
      for (_, client) in server.clients.lock().unwrap().iter_mut() {
        client.send(protocol::ServerToClient::UpdateSun(fraction));
      }
    });
  });
}

fn translate_mob(
  server: &Server,
  mob: &mut mob::Mob,
  delta_p: &Vector3<f32>,
) {
  let bounds;
  {
    let mut physics = server.physics.lock().unwrap();
    if physics.translate_misc(mob.entity_id, *delta_p).is_some() {
      mob.speed.add_self_v(&delta_p.neg());
      return;
    } else {
      bounds = physics.get_bounds(mob.entity_id).unwrap().clone();
    }
  }

  mob.position.add_self_v(delta_p);

  for (_, client) in server.clients.lock().unwrap().iter_mut() {
    client.send(
      protocol::ServerToClient::UpdateMob(mob.entity_id, bounds.clone())
    );
  }
}

pub fn load_placeholders<RequestBlock>(
  owner: lod::OwnerId,
  server: &Server,
  request_block: &mut RequestBlock,
  pos: &voxel::bounds::T,
  load_type: LoadType,
) where
  RequestBlock: FnMut(update_gaia::Message),
{
  match load_type {
    LoadType::Load | LoadType::Update => {
      server.terrain_loader.load(
        &server.id_allocator,
        &server.physics,
        &pos,
        lod::Placeholder,
        owner,
        request_block,
      );
    },
    LoadType::Unload => {
      server.terrain_loader.unload(
        &server.physics,
        &pos,
        owner,
      );
    },
  }
}
