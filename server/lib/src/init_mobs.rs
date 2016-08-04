use cgmath::{Point3, EuclideanSpace, InnerSpace, Vector3};
use collision::{Aabb3};

use common::entity_id;
use common::id_allocator;
use common::surroundings_loader;

use mob;
use server;

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  (bounds.min + bounds.max.to_vec()) * 0.5
}

// TODO: Locking is hard to reason about. Make it saner.
// The goal should be to prevent coder error causing deadlock.

pub fn init_mobs(
  server: &server::T,
) {
  fn mob_behavior(world: &server::T, mob: &mut mob::Mob) {
    fn to_player(world: &server::T, mob: &mob::Mob) -> Option<Vector3<f32>> {
      let mob_posn = center(world.physics.lock().unwrap().get_bounds(mob.entity_id).unwrap());

      let players: Vec<entity_id::T> = world.players.lock().unwrap().keys().cloned().collect();
      let mut players = players.into_iter();

      players.next().map(|id| {
        let mut min_v = center(world.physics.lock().unwrap().get_bounds(id).unwrap()) - mob_posn;
        let mut min_d = min_v.magnitude2();
        for id in players {
          let v = center(world.physics.lock().unwrap().get_bounds(id).unwrap()) - mob_posn;
          let d = v.magnitude2();
          if d < min_d {
            min_v = v;
            min_d = d;
          }
        }

        min_v
      })
    }

    {
      match to_player(world, mob) {
        None => { mob.behavior = mob_behavior },
        Some(to_player) => {
          if to_player.magnitude() < 2.0 {
            mob.behavior = wait_for_distance;
          }
        },
      }
    }

    fn wait_for_distance(world: &server::T, mob: &mut mob::Mob) {
      match to_player(world, mob) {
        None => { mob.behavior = mob_behavior },
        Some(to_player) => {
          if to_player.magnitude() > 8.0 {
            mob.behavior = follow_player;
          }
        },
      }
    }

    fn follow_player(world: &server::T, mob: &mut mob::Mob) {
      match to_player(world, mob) {
        None => { mob.behavior = mob_behavior },
        Some(to_player) => {
          if to_player.magnitude2() < 4.0 {
            mob.behavior = wait_to_reset;
            mob.speed = Vector3::new(0.0, 0.0, 0.0);
          } else {
            mob.speed = to_player * (0.5);
          }
        },
      }
    }

    fn wait_to_reset(world: &server::T, mob: &mut mob::Mob) {
      match to_player(world, mob) {
        None => { mob.behavior = mob_behavior },
        Some(to_player) => {
          if to_player.magnitude() >= 2.0 {
            mob.behavior = mob_behavior;
          }
        },
      }
    }
  }

  add_mob(
    server,
    // TODO: shift upward until outside terrain
    Point3::new(0.0, 64.0, -1.0),
    mob_behavior,
  );
}

fn add_mob(
  server: &server::T,
  low_corner: Point3<f32>,
  behavior: mob::Behavior,
) {
  let bounds = Aabb3::new(low_corner, low_corner + (&Vector3::new(1.0, 2.0, 1.0 as f32)));
  let entity_id = id_allocator::allocate(&server.id_allocator);

  let mob =
    mob::Mob {
      position: (bounds.min + bounds.max.to_vec()) * 0.5,
      speed: Vector3::new(0.0, 0.0, 0.0),
      behavior: behavior,
      entity_id: entity_id,
      owner_id: id_allocator::allocate(&server.owner_allocator),
      surroundings_loader: surroundings_loader::new(8, Vec::new()),
    };

  server.physics.lock().unwrap().insert_misc(entity_id, &bounds);
  server.mobs.lock().unwrap().insert(entity_id, mob);
}
