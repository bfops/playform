use color::Color4;
use common::*;
use gl::types::*;
use mob;
use nalgebra::Vec3;
use opencl_context::CL;
use physics::Physics;
use world::World;
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Sender;
use stopwatch::TimerSet;
use terrain::terrain_block::BlockPosition;
use view_update::ViewUpdate;
use view_update::ViewUpdate::*;

pub fn update(
  timers: &TimerSet,
  world: &mut World,
  view: &Sender<ViewUpdate>,
  cl: &CL,
) {
  timers.time("update", || {
    timers.time("update.player", || {
      world.player.update(
        timers,
        view,
        cl,
        &mut world.terrain_game_loader,
        &mut world.id_allocator,
        &mut world.physics,
      );

      view.send(UpdatePlayer(world.player.position)).unwrap();
    });

    timers.time("update.mobs", || {
      for (_, mob) in world.mobs.iter() {
        let mut mob_cell = mob.deref().borrow_mut();
        let mob = mob_cell.deref_mut();

        let block_position = BlockPosition::from_world_position(&mob.position);

        mob.solid_boundary.update(
          timers,
          view,
          cl,
          &mut world.terrain_game_loader,
          &mut world.id_allocator,
          &mut world.physics,
          block_position,
        );

        {
          let behavior = mob.behavior;
          (behavior)(world, mob);
        }

        mob.speed = mob.speed - Vec3::new(0.0, 0.1, 0.0 as GLfloat);

        macro_rules! translate_mob(
          ($v:expr) => (
            translate_mob(
              view,
              &mut world.physics,
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

    world.sun.update().map(|fraction| {
      view.send(UpdateSun(fraction)).unwrap();
    });
  })
}

fn translate_mob(
  view: &Sender<ViewUpdate>,
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
    view.send(UpdateMob((mob.id, vec))).unwrap();
  }
}
