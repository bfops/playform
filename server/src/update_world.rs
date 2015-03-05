use cgmath::{Point, Vector, Vector3};
use std::ops::Neg;
use std::sync::mpsc::Sender;

use common::block_position::BlockPosition;
use common::color::Color4;
use common::communicate::ServerToClient::*;
use common::lod::{LOD, OwnerId};
use common::stopwatch::TimerSet;
use common::surroundings_loader::LODChange;

use mob;
use server::Server;
use update_gaia::ServerToGaia;

// TODO: Consider removing the IntervalTimer.

pub fn update_world(
  timers: &TimerSet,
  server: &Server,
  request_block: &Sender<ServerToGaia>,
) {
  let mut request_block = |block| { request_block.send(block).unwrap() };

  timers.time("update", || {
    timers.time("update.player", || {
      server.player.lock().unwrap().update(timers, server, &mut request_block);

      let player_position = server.player.lock().unwrap().position;
      trace!("player_position {:?}", player_position);
      server.to_client.lock().unwrap().as_mut().map(|&mut (ref client, _)| {
        client.send(Some(UpdatePlayer(player_position))).unwrap();
      });
    });

    timers.time("update.mobs", || {
      for (_, mob) in server.mobs.lock().unwrap().iter_mut() {
        let block_position = BlockPosition::from_world_position(&mob.position);

        let owner_id = mob.owner_id;
        mob.surroundings_loader.update(
          block_position,
          |lod_change|
            load_placeholders(
              timers,
              owner_id,
              server,
              &mut request_block,
              lod_change,
            )
        );

        {
          let behavior = mob.behavior;
          (behavior)(server, mob);
        }

        mob.speed = mob.speed - Vector3::new(0.0, 0.1, 0.0 as f32);

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
      server.to_client.lock().unwrap().as_mut().map(|&mut (ref client, _)| {
        client.send(Some(UpdateSun(fraction))).unwrap();
      });
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

  // TODO: Just send new position. Mesh remains the same.
  server.to_client.lock().unwrap().as_ref().map(|&(ref client, _)| {
    let vec =
      mob::Mob::to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
      .iter()
      .map(|&x| x)
      .collect();
    client.send(Some(UpdateMob(mob.entity_id, vec))).unwrap();
  });
}

#[inline]
pub fn load_placeholders<RequestBlock>(
  timers: &TimerSet,
  owner: OwnerId,
  server: &Server,
  request_block: &mut RequestBlock,
  lod_change: LODChange,
) where
  RequestBlock: FnMut(ServerToGaia),
{
  match lod_change {
    LODChange::Load(pos, _) => {
      server.terrain_game_loader.lock().unwrap().load(
        timers,
        &server.id_allocator,
        &server.physics,
        &pos,
        LOD::Placeholder,
        owner,
        request_block,
      );
    },
    LODChange::Unload(pos) => {
      server.terrain_game_loader.lock().unwrap().unload(
        timers,
        &server.physics,
        &pos,
        owner,
      );
    },
  }
}
