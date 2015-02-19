use common::cube_shell::cube_diff;
use common::entity::EntityId;
use mob;
use nalgebra::{Vec3, Pnt3, Norm};
use nalgebra;
use ncollide_entities::bounding_volume::{AABB, AABB3};
use std::collections::HashMap;
use common::surroundings_loader::SurroundingsLoader;
use terrain::terrain;
use server::Server;

fn center(bounds: &AABB3<f32>) -> Pnt3<f32> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as f32)
}

pub fn init_mobs<'a>(
  server: &Server,
  mob_loaders: &mut HashMap<EntityId, SurroundingsLoader<'a>>,
) {
  fn mob_behavior(world: &Server, mob: &mut mob::Mob) {
    fn to_player(world: &Server, mob: &mob::Mob) -> Vec3<f32> {
      let physics = world.physics.lock().unwrap();

      center(physics.get_bounds(world.player.lock().unwrap().entity_id).unwrap()) -
      center(physics.get_bounds(mob.entity_id).unwrap())
    }

    {
      let to_player = to_player(world, mob);
      if nalgebra::norm(&to_player) < 2.0 {
        mob.behavior = wait_for_distance;
      }
    }

    fn wait_for_distance(world: &Server, mob: &mut mob::Mob) {
      let to_player = to_player(world, mob);
      if nalgebra::norm(&to_player) > 8.0 {
        mob.behavior = follow_player;
      }
    }

    fn follow_player(world: &Server, mob: &mut mob::Mob) {
      let to_player = to_player(world, mob);
      if to_player.sqnorm() < 4.0 {
        mob.behavior = wait_to_reset;
        mob.speed = Vec3::new(0.0, 0.0, 0.0);
      } else {
        mob.speed = to_player / 2.0 as f32;
      }
    }

    fn wait_to_reset(world: &Server, mob: &mut mob::Mob) {
      let to_player = to_player(world, mob);
      if nalgebra::norm(&to_player) >= 2.0 {
        mob.behavior = mob_behavior;
      }
    }
  }

  add_mob(
    server,
    mob_loaders,
    Pnt3::new(0.0, terrain::AMPLITUDE as f32, -1.0),
    mob_behavior,
  );
}

fn add_mob<'a>(
  server: &Server,
  loaders: &mut HashMap<EntityId, SurroundingsLoader<'a>>,
  low_corner: Pnt3<f32>,
  behavior: mob::Behavior,
) {
  let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as f32));
  let entity_id = server.id_allocator.lock().unwrap().allocate();

  let mob =
    mob::Mob {
      position: (*bounds.mins() + bounds.maxs().to_vec()) / 2.0,
      speed: Vec3::new(0.0, 0.0, 0.0),
      behavior: behavior,
      entity_id: entity_id,
      owner_id: server.owner_allocator.lock().unwrap().allocate(),
    };

  loaders.insert(
    entity_id,
    SurroundingsLoader::new(
      1,
      Box::new(|&: last, cur| cube_diff(last, cur, 1)),
    ),
  );

  server.physics.lock().unwrap().insert_misc(entity_id, bounds);
  server.mobs.lock().unwrap().insert(entity_id, mob);
}
