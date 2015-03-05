use cgmath::{Aabb3, Point, Point3, EuclideanVector, Vector, Vector3};
use common::surroundings_loader::SurroundingsLoader;
use mob;
use server::Server;
use terrain::terrain;

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(0.5)
}

// TODO: Locking is hard to reason about. Make it saner.
// The goal should be to prevent coder error causing deadlock.

pub fn init_mobs(
  server: &Server,
) {
  fn mob_behavior(world: &Server, mob: &mut mob::Mob) {
    fn to_player(world: &Server, mob: &mob::Mob) -> Vector3<f32> {
      let player = world.player.lock().unwrap().entity_id;
      let physics = world.physics.lock().unwrap();

      center(physics.get_bounds(player).unwrap())
        .sub_p(&center(physics.get_bounds(mob.entity_id).unwrap()))
    }

    {
      let to_player = to_player(world, mob).length();
      if to_player < 2.0 {
        mob.behavior = wait_for_distance;
      }
    }

    fn wait_for_distance(world: &Server, mob: &mut mob::Mob) {
      let to_player = to_player(world, mob);
      if to_player.length() > 8.0 {
        mob.behavior = follow_player;
      }
    }

    fn follow_player(world: &Server, mob: &mut mob::Mob) {
      let to_player = to_player(world, mob);
      if to_player.length2() < 4.0 {
        mob.behavior = wait_to_reset;
        mob.speed = Vector3::new(0.0, 0.0, 0.0);
      } else {
        mob.speed = to_player.mul_s(0.5);
      }
    }

    fn wait_to_reset(world: &Server, mob: &mut mob::Mob) {
      let to_player = to_player(world, mob);
      if to_player.length() >= 2.0 {
        mob.behavior = mob_behavior;
      }
    }
  }

  add_mob(
    server,
    Point3::new(0.0, terrain::AMPLITUDE as f32, -1.0),
    mob_behavior,
  );
}

fn add_mob(
  server: &Server,
  low_corner: Point3<f32>,
  behavior: mob::Behavior,
) {
  let bounds = Aabb3::new(low_corner, low_corner.add_v(&Vector3::new(1.0, 2.0, 1.0 as f32)));
  let entity_id = server.id_allocator.lock().unwrap().allocate();

  let mob =
    mob::Mob {
      position: bounds.min.add_v(&bounds.max.to_vec()).mul_s(0.5),
      speed: Vector3::new(0.0, 0.0, 0.0),
      behavior: behavior,
      entity_id: entity_id,
      owner_id: server.owner_allocator.lock().unwrap().allocate(),
      surroundings_loader: SurroundingsLoader::new(1, Vec::new()),
    };

  server.physics.lock().unwrap().insert_misc(entity_id, bounds);
  server.mobs.lock().unwrap().insert(entity_id, mob);
}
