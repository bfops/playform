use color::Color4;
use common::*;
use gl::types::*;
use loader::Operation;
use mob;
use nalgebra::Vec3;
use physics::Physics;
use state::EntityId;
use state::App;
use std::collections::HashMap;
use terrain::Terrain;
use yaglw::gl_context::GLContext;

static OCTREE_LOAD_SPEED: uint = 1 << 11;

macro_rules! translate_mob(
  ($world:expr, $mob:expr, $v:expr) => (
    translate_mob(
      $world.gl_context,
      &mut $world.physics,
      &mut $world.mob_buffers,
      $mob,
      $v
    );
  );
);

pub fn update<'a>(app: &mut App) {
  app.timers.time("update", || {
    let player_block_position = Terrain::to_block_position(app.player.camera.position);

    app.timers.time("update.load", || {
      app.timers.time("update.load.terrain", || {
        app.surroundings_loader.update(
          app.timers,
          app.gl_context,
          &mut app.terrain_buffers,
          &mut app.id_allocator,
          &mut app.physics,
          player_block_position,
        );
      });
      app.timers.time("update.load.octree", || {
        load_octree(app);
      });
    });

    app.timers.time("update.player", || {
      if app.surroundings_loader.loaded.contains(&player_block_position) {
        app.player.update(&mut app.physics);
      }
    });

    app.timers.time("update.mobs", || {
      // Unsafely mutably borrow the mobs.
      let mobs: *mut HashMap<EntityId, mob:: Mob> = &mut app.mobs;
      for (_, mob) in unsafe { (*mobs).iter_mut() } {
        // Please don't do sketchy things with the `mobs` vector. The first time the
        // unsafety here bites us, it should be replaced with runtime checks.

        let block_position = Terrain::to_block_position(mob.position);

        if app.surroundings_loader.loaded.contains(&block_position) {
          {
            let behavior = mob.behavior;
            unsafe { (behavior)(app, mob); }
          }

          mob.speed = mob.speed - Vec3::new(0.0, 0.1, 0.0 as GLfloat);

          let delta_p = mob.speed;
          if delta_p.x != 0.0 {
            translate_mob!(app, mob, Vec3::new(delta_p.x, 0.0, 0.0));
          }
          if delta_p.y != 0.0 {
            translate_mob!(app, mob, Vec3::new(0.0, delta_p.y, 0.0));
          }
          if delta_p.z != 0.0 {
            translate_mob!(app, mob, Vec3::new(0.0, 0.0, delta_p.z));
          }
        }
      }
    });
  })
}
 
fn translate_mob(
  gl: &mut GLContext,
  physics: &mut Physics<EntityId>,
  mob_buffers: &mut mob::MobBuffers,
  mob: &mut mob::Mob,
  delta_p: Vec3<GLfloat>,
) {
  if physics.translate(mob.id, delta_p).is_some() {
    mob.speed = mob.speed - delta_p;
  } else {
    let bounds = physics.get_bounds(mob.id).unwrap();
    mob_buffers.update(
      gl,
      mob.id,
      &to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
    );
  }
}

fn load_octree<'a>(app: &mut App<'a>) {
  // octree loading
  for _ in range(0, OCTREE_LOAD_SPEED) {
    match app.octree_loader.borrow_mut().pop_front() {
      None => break,
      Some(Operation::Load((id, bounds))) => {
        app.octree_buffers.push(app.gl_context, id, &to_outlines(&bounds));
      },
      Some(Operation::Unload(id)) => {
        app.octree_buffers.swap_remove(app.gl_context, id);
      }
    }
  }
}
