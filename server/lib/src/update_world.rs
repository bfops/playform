use cgmath::{Point3, Vector3};
use std::ops::Neg;
use stopwatch;

use common::protocol;
use common::surroundings_loader::LoadType;
use common::voxel;

use lod;
use mob;
use player;
use server;
use update_gaia;

// TODO: Consider removing the IntervalTimer.

pub fn update_world<RequestBlock>(
  server: &server::T,
  request_block: &mut RequestBlock,
) where
  RequestBlock: FnMut(update_gaia::Message),
{
  stopwatch::time("update_world", || {
    stopwatch::time("update_world.player", || {
      let mut updates = Vec::new();

      for (_, player) in server.players.lock().unwrap().iter_mut() {
        let (bounds, collisions) = player.update(server, request_block);
        updates.push(protocol::ServerToClient::UpdatePlayer(player.entity_id, bounds));
        updates.extend(
          collisions.into_iter()
          .map(|c| {
            match c {
              player::Collision::Terrain(_) => protocol::Collision::PlayerTerrain(player.entity_id),
              player::Collision::Misc(_)    => protocol::Collision::PlayerMisc(player.entity_id),
            }
          })
          .map(|c| {
            protocol::ServerToClient::Collision(c)
          })
        );
      }

      let mut clients = server.clients.lock().unwrap();
      for (_, client) in &mut *clients {
        for update in &updates {
          client.send(update.clone());
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

        mob.speed = mob.speed + -Vector3::new(0.0, 0.1, 0.0 as f32);

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
  server: &server::T,
  mob: &mut mob::Mob,
  delta_p: &Vector3<f32>,
) {
  let bounds;
  {
    let mut physics = server.physics.lock().unwrap();
    if physics.translate_misc(mob.physics_id, *delta_p).is_some() {
      mob.speed += delta_p.neg();
      return;
    } else {
      bounds = *physics.get_bounds(mob.physics_id).unwrap();
    }
  }

  mob.position += *delta_p;

  for (_, client) in server.clients.lock().unwrap().iter_mut() {
    client.send(
      protocol::ServerToClient::UpdateMob(mob.entity_id, bounds),
    );
  }
}

pub fn load_placeholders<RequestBlock>(
  owner: lod::OwnerId,
  server: &server::T,
  request_block: &mut RequestBlock,
  pos: &voxel::bounds::T,
  load_type: LoadType,
) where
  RequestBlock: FnMut(update_gaia::Message),
{
  match load_type {
    LoadType::Load | LoadType::Downgrade => {
      server.terrain_loader.load(
        &server.misc_allocator,
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
