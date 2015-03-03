use cgmath::{Point, Vector, Vector3};
use std::collections::HashMap;
use std::ops::Neg;
use std::thread;
use time;

use common::block_position::BlockPosition;
use common::color::Color4;
use common::communicate::ClientToServer;
use common::communicate::ServerToClient::*;
use common::entity::EntityId;
use common::interval_timer::IntervalTimer;
use common::lod::{LOD, LODIndex, OwnerId};
use common::stopwatch::TimerSet;
use common::surroundings_loader::{SurroundingsLoader, LODChange};
use common::terrain_block::{BLOCK_WIDTH, TEXTURE_WIDTH};

use client_recv_thread::apply_client_update;
use init_mobs::init_mobs;
use mob;
use opencl_context::CL;
use server::Server;
use sun::Sun;
use terrain::texture_generator::TerrainTextureGenerator;
use update_gaia::{ServerToGaia, update_gaia};

// TODO: Consider removing the IntervalTimer.

const UPDATES_PER_SECOND: u64 = 30;
const SUN_TICK_NS: u64 = 5000000;

pub fn update_thread<RecvClient, RecvGaia, RequestBlock>(
  server: &Server,
  recv_client: &mut RecvClient,
  recv_gaia: &mut RecvGaia,
  request_block: &mut RequestBlock,
) where
  RecvClient: FnMut() -> Option<ClientToServer>,
  RecvGaia: FnMut() -> Option<ServerToGaia>,
  RequestBlock: FnMut(ServerToGaia),
{
  let timers = TimerSet::new();
  let timers = &timers;

  let cl = unsafe {
    CL::new()
  };
  let cl = &cl;

  let texture_generators = [
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[0], BLOCK_WIDTH as u32),
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[1], BLOCK_WIDTH as u32),
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[2], BLOCK_WIDTH as u32),
    TerrainTextureGenerator::new(&cl, TEXTURE_WIDTH[3], BLOCK_WIDTH as u32),
  ];

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
    if let Some(update) = recv_client() {
      println!("{} recv client: {:?}", time::precise_time_ns(), update);
      apply_client_update(server, request_block, update);
    } else {
      if update_timer.update(time::precise_time_ns()) > 0 {
        println!("{} update", time::precise_time_ns());
        update_world(
          timers,
          server,
          &mut sun,
          request_block,
          &mut player_surroundings_loader,
          player_surroundings_owner,
          &mut player_solid_boundary,
          player_solid_owner,
          &mut mob_loaders,
        );
      } else {
        if let Some(update) = recv_gaia() {
          println!("{} update gaia", time::precise_time_ns());
          update_gaia(
            timers,
            &server,
            &texture_generators,
            cl,
            update,
          );
        } else {
          thread::yield_now();
        }
      }
    }
  }
}

fn update_world<RequestBlock>(
  timers: &TimerSet,
  server: &Server,
  sun: &mut Sun,
  request_block: &mut RequestBlock,
  player_surroundings_loader: &mut SurroundingsLoader,
  player_surroundings_owner: OwnerId,
  player_solid_boundary: &mut SurroundingsLoader,
  player_solid_owner: OwnerId,
  mob_loaders: &mut HashMap<EntityId, SurroundingsLoader>,
) where
  RequestBlock: FnMut(ServerToGaia),
{
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
                  request_block,
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
              request_block,
              lod_change,
            )
        );
      });

      server.player.lock().unwrap().update(server);

      let player_position = server.player.lock().unwrap().position;
      server.to_client.lock().unwrap().as_mut().map(|&mut (ref client, _)| {
        client.send(Some(UpdatePlayer(player_position))).unwrap();
        println!("{} update player {:?}", time::precise_time_ns(), player_position);
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
                request_block,
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
