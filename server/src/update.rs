use common::block_position::BlockPosition;
use common::color::Color4;
use common::communicate::ServerToClient;
use common::communicate::ServerToClient::*;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LOD;
use common::stopwatch::TimerSet;
use common::surroundings_loader::LODChange;
use gaia_update::ServerToGaia;
use mob;
use nalgebra::Vec3;
use physics::Physics;
use server::Server;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Sender;
use terrain::terrain_game_loader::TerrainGameLoader;

pub fn update(
  timers: &TimerSet,
  server: &mut Server,
  ups_to_client: &Sender<ServerToClient>,
  ups_to_gaia: &Sender<ServerToGaia>,
) {
  timers.time("update", || {
    timers.time("update.player", || {
      server.player.update(
        timers,
        &mut server.terrain_game_loader,
        &mut server.id_allocator,
        &mut server.physics,
        ups_to_gaia,
      );

      ups_to_client.send(UpdatePlayer(server.player.position)).unwrap();
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
            |lod_change|
              load_placeholders(
                timers,
                id_allocator,
                physics,
                terrain_game_loader,
                ups_to_gaia,
                lod_change,
              )
          );
        }

        {
          let behavior = mob.behavior;
          (behavior)(server, mob);
        }

        mob.speed = mob.speed - Vec3::new(0.0, 0.1, 0.0 as f32);

        macro_rules! translate_mob(
          ($v:expr) => (
            translate_mob(
              ups_to_client,
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
      ups_to_client.send(UpdateSun(fraction)).unwrap();
    });
  });
}

fn translate_mob(
  ups_to_client: &Sender<ServerToClient>,
  physics: &mut Physics,
  mob: &mut mob::Mob,
  delta_p: Vec3<f32>,
) {
  if physics.translate_misc(mob.id, delta_p).is_some() {
    mob.speed = mob.speed - delta_p;
  } else {
    let bounds = physics.get_bounds(mob.id).unwrap();
    mob.position = mob.position + delta_p;

    let vec =
      mob::Mob::to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
      .iter()
      .map(|&x| x)
      .collect();
    ups_to_client.send(UpdateMob(mob.id, vec)).unwrap();
  }
}

#[inline]
pub fn load_placeholders(
  timers: &TimerSet,
  id_allocator: &mut IdAllocator<EntityId>,
  physics: &mut Physics,
  terrain_game_loader: &mut TerrainGameLoader,
  ups_to_gaia: &Sender<ServerToGaia>,
  lod_change: LODChange,
) {
  match lod_change {
    LODChange::Load(pos, _, id) => {
      terrain_game_loader.load(
        timers,
        id_allocator,
        physics,
        &pos,
        LOD::Placeholder,
        id,
        ups_to_gaia,
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
  }
}
