use cube_shell::cube_diff;
use color::Color4;
use common::*;
use gl::types::*;
use id_allocator::IdAllocator;
use lod_map::{LOD, OwnerId};
use mob;
use nalgebra::{Vec3, Pnt3, Norm};
use nalgebra;
use ncollide_entities::bounding_volume::{AABB, AABB3};
use physics::Physics;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use surroundings_loader::SurroundingsLoader;
use terrain::terrain;
use view_update::ViewUpdate;
use view_update::ViewUpdate::*;
use world::{World, EntityId};

fn center(bounds: &AABB3<f32>) -> Pnt3<GLfloat> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as GLfloat)
}

pub fn make_mobs<'a>(
  view: &Sender<ViewUpdate>,
  physics: &mut Physics,
  id_allocator: &mut IdAllocator<EntityId>,
  owner_allocator: &mut IdAllocator<OwnerId>,
) -> HashMap<EntityId, Rc<RefCell<mob::Mob<'a>>>> {
  let mut mobs = HashMap::new();

  fn mob_behavior(world: &World, mob: &mut mob::Mob) {
    let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
    if nalgebra::norm(&to_player) < 2.0 {
      mob.behavior = wait_for_distance;
    }

    fn wait_for_distance(world: &World, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if nalgebra::norm(&to_player) > 8.0 {
        mob.behavior = follow_player;
      }
    }

    fn follow_player(world: &World, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if to_player.sqnorm() < 4.0 {
        mob.behavior = wait_to_reset;
        mob.speed = Vec3::new(0.0, 0.0, 0.0);
      } else {
        mob.speed = to_player / 2.0 as GLfloat;
      }
    }

    fn wait_to_reset(world: &World, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if nalgebra::norm(&to_player) >= 2.0 {
        mob.behavior = mob_behavior;
      }
    }
  }

  add_mob(
    view,
    physics,
    &mut mobs,
    id_allocator,
    owner_allocator,
    Pnt3::new(0.0, terrain::AMPLITUDE as f32, -1.0),
    mob_behavior
  );

  mobs
}

fn add_mob(
  view: &Sender<ViewUpdate>,
  physics: &mut Physics,
  mobs: &mut HashMap<EntityId, Rc<RefCell<mob::Mob>>>,
  id_allocator: &mut IdAllocator<EntityId>,
  owner_allocator: &mut IdAllocator<OwnerId>,
  low_corner: Pnt3<GLfloat>,
  behavior: mob::Behavior,
) {
  // TODO: mob loader instead of pushing directly to gl buffers

  let id = id_allocator.allocate();
  let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as GLfloat));

  let mob =
    mob::Mob {
      position: (*bounds.mins() + bounds.maxs().to_vec()) / 2.0,
      speed: Vec3::new(0.0, 0.0, 0.0),
      behavior: behavior,
      id: id,
      solid_boundary:
        SurroundingsLoader::new(
          owner_allocator.allocate(),
          1,
          Box::new(|&: _| LOD::Placeholder),
          Box::new(|&: last, cur| cube_diff(last, cur, 1)),
        ),
    };
  let mob = Rc::new(RefCell::new(mob));

  let triangles =
    to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
    .iter()
    .map(|&x| x)
    .collect();
  view.send(PushMob((id, triangles))).unwrap();

  physics.insert_misc(id, bounds);
  mobs.insert(id, mob);
}
