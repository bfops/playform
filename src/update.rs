use client_update::ServerToClient;
use client_update::ServerToClient::*;
use color::Color4;
use common::*;
use gl::types::*;
use mob;
use nalgebra::Vec3;
use opencl_context::CL;
use physics::Physics;
use server::Server;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Sender;
use stopwatch::TimerSet;
use surroundings_loader::LODChange;
use terrain::terrain_block::BlockPosition;

pub fn update(
  timers: &TimerSet,
  server: &mut Server,
  client: &Sender<ServerToClient>,
  cl: &CL,
) {
  timers.time("update", || {
    timers.time("update.player", || {
      server.player.update(
        timers,
        cl,
        &mut server.terrain_game_loader,
        &mut server.id_allocator,
        &mut server.physics,
      );

      client.send(UpdatePlayer(server.player.position)).unwrap();
    });

    timers.time("update.mobs", || {
      for (_, mob) in server.mobs.iter() {
        let mut mob_cell = mob.deref().borrow_mut();
        let mob = mob_cell.deref_mut();

        let block_position = BlockPosition::from_world_position(&mob.position);

        {
          let terrain_game_loader = &mut server.terrain_game_loader;
          let id_allocator = &mut server.id_allocator;
          let physics = &mut server.physics;
          mob.solid_boundary.update(
            block_position,
            |lod_change| {
              match lod_change {
                LODChange::Load(pos, lod, id) => {
                  terrain_game_loader.load(
                    timers,
                    cl,
                    id_allocator,
                    physics,
                    &pos,
                    lod,
                    id,
                  );
                },
                LODChange::Unload(pos, id) => {
                  terrain_game_loader.unload(
                    timers,
                    physics,
                    &pos,
                    id,
                  );
                },
              };
            },
          );
        }

        {
          let behavior = mob.behavior;
          (behavior)(server, mob);
        }

        mob.speed = mob.speed - Vec3::new(0.0, 0.1, 0.0 as GLfloat);

        macro_rules! translate_mob(
          ($v:expr) => (
            translate_mob(
              client,
              &mut server.physics,
              mob,
              $v
            );
          );
        );

        let delta_p = mob.speed;
        if delta_p.x != 0.0 {
          translate_mob!(Vec3::new(delta_p.x, 0.0, 0.0));
        }
        if delta_p.y != 0.0 {
          translate_mob!(Vec3::new(0.0, delta_p.y, 0.0));
        }
        if delta_p.z != 0.0 {
          translate_mob!(Vec3::new(0.0, 0.0, delta_p.z));
        }
      }
    });

    server.sun.update().map(|fraction| {
      client.send(UpdateSun(fraction)).unwrap();
    });
  })
}

fn translate_mob(
  client: &Sender<ServerToClient>,
  physics: &mut Physics,
  mob: &mut mob::Mob,
  delta_p: Vec3<GLfloat>,
) {
  if physics.translate_misc(mob.id, delta_p).is_some() {
    mob.speed = mob.speed - delta_p;
  } else {
    let bounds = physics.get_bounds(mob.id).unwrap();
    mob.position = mob.position + delta_p;

    let vec =
      to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
      .iter()
      .map(|&x| x)
      .collect();
    client.send(UpdateMob((mob.id, vec))).unwrap();
  }
}
