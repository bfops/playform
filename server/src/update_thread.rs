use cgmath::{Point, Vector, Vector3};
use common::block_position::BlockPosition;
use common::color::Color4;
use common::communicate::ServerToClient::*;
use common::interval_timer::IntervalTimer;
use common::lod::{LOD, LODIndex, OwnerId};
use common::stopwatch::TimerSet;
use common::surroundings_loader::{SurroundingsLoader, LODChange};
use init_mobs::init_mobs;
use mob;
use server::Server;
use std::collections::HashMap;
use std::old_io::timer;
use std::ops::Neg;
use std::time::duration::Duration;
use sun::Sun;
use time;
use update_gaia::ServerToGaia;

// TODO: Consider removing the IntervalTimer.

const UPDATES_PER_SECOND: u64 = 30;
const SUN_TICK_NS: u64 = 5000000;

pub fn update_thread<UpdateGaia>(
  server: &Server,
  update_gaia: &mut UpdateGaia,
) where
  UpdateGaia: FnMut(ServerToGaia),
{
  let timers = TimerSet::new();
  let timers = &timers;

  let mut mob_loaders = HashMap::new();
  timers.time("init_mobs", || {
    init_mobs(server, &mut mob_loaders);
  });

  let mut update_timer;
  {
    let now = time::precise_time_ns();
    let nanoseconds_per_second = 1000000000;
    update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
  }

  let mut sun = Sun::new(SUN_TICK_NS);

  // TODO: Make a struct for these.
  let player_surroundings_owner = server.owner_allocator.lock().unwrap().allocate();
  let player_solid_owner = server.owner_allocator.lock().unwrap().allocate();
  let mut player_surroundings_loader = SurroundingsLoader::new(1, Vec::new());
  // Nearby blocks should be made solid if they aren't loaded yet.
  let mut player_solid_boundary = SurroundingsLoader::new(1, Vec::new());

  loop {
    let updates = update_timer.update(time::precise_time_ns());
    if updates > 0 {
      timers.time("update", || {
        timers.time("update.player", || {
          let block_position = BlockPosition::from_world_position(&server.player.lock().unwrap().position);

          timers.time("update.player.surroundings", || {
            player_surroundings_loader.update(
              block_position,
              |lod_change| {
                match lod_change {
                  LODChange::Load(pos, _) => {
                    server.terrain_game_loader.lock().unwrap().load(
                      timers,
                      &server.id_allocator,
                      &server.physics,
                      &pos,
                      LOD::LodIndex(LODIndex(0)),
                      player_surroundings_owner,
                      update_gaia,
                    );
                  },
                  LODChange::Unload(pos) => {
                    server.terrain_game_loader.lock().unwrap().unload(
                      timers,
                      &server.physics,
                      &pos,
                      player_surroundings_owner,
                    );
                  },
                }
              },
            );

            player_solid_boundary.update(
              block_position,
              |lod_change|
                load_placeholders(
                  timers,
                  player_solid_owner,
                  server,
                  update_gaia,
                  lod_change,
                )
            );
          });

          server.player.lock().unwrap().update(server);

          let player_position = server.player.lock().unwrap().position;
          server.to_client.lock().unwrap().as_mut().map(|&mut (ref client, _)| {
            client.send(Some(UpdatePlayer(player_position))).unwrap();
          });
        });

        timers.time("update.mobs", || {
          for (id, mob) in server.mobs.lock().unwrap().iter_mut() {
            let block_position = BlockPosition::from_world_position(&mob.position);

            {
              mob_loaders.get_mut(id).unwrap().update(
                block_position,
                |lod_change|
                  load_placeholders(
                    timers,
                    mob.owner_id,
                    server,
                    update_gaia,
                    lod_change,
                  )
              );
            }

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

        sun.update().map(|fraction| {
          server.to_client.lock().unwrap().as_mut().map(|&mut (ref client, _)| {
            client.send(Some(UpdateSun(fraction))).unwrap();
          });
        });
      });
    }

    timer::sleep(Duration::milliseconds(1));
  }
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
pub fn load_placeholders<UpdateGaia>(
  timers: &TimerSet,
  owner: OwnerId,
  server: &Server,
  update_gaia: &mut UpdateGaia,
  lod_change: LODChange,
) where UpdateGaia: FnMut(ServerToGaia) {
  match lod_change {
    LODChange::Load(pos, _) => {
      server.terrain_game_loader.lock().unwrap().load(
        timers,
        &server.id_allocator,
        &server.physics,
        &pos,
        LOD::Placeholder,
        owner,
        update_gaia,
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
