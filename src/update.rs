use common::*;
use gl::types::*;
use glw::color::Color4;
use input;
use loader::{Load, Unload};
use mob;
use nalgebra::Vec3;
use physics::Physics;
use state::EntityId;
use state::App;
use stopwatch;
use stopwatch::*;
use std::cmp;
use std::collections::HashMap;

// how many terrain polys to load during every update step
static TERRAIN_LOAD_SPEED: uint = 1 << 10;
static OCTREE_LOAD_SPEED: uint = 1 << 11;

macro_rules! translate_mob(
  ($world:expr, $mob:expr, $v:expr) => (
    translate_mob(
      &mut $world.physics,
      &mut $world.mob_buffers,
      $mob,
      $v
    );
  );
)

pub fn update<'a>(app: &mut App) {
  time!(app.timers.deref(), "update", || {
    // TODO(cgaebel): Ideally, the update thread should not be touching OpenGL.

    time!(app.timers.deref(), "update.load", || {
      load_terrain(app, Some(TERRAIN_LOAD_SPEED));
      load_octree(app);
    });

    time!(app.timers.deref(), "update.player", || {
      app.player.update(&mut app.physics);
    });

    time!(app.timers.deref(), "update.mobs", || {
      // Unsafely mutably borrow the mobs.
      let mobs: *mut HashMap<EntityId, mob:: Mob> = &mut app.mobs;
      for (_, mob) in unsafe { (*mobs).iter_mut() } {
        // Please don't do sketchy things with the `mobs` vector.
        // The first time the unsafety here bites us, it should be replaced
        // with runtime checks.

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
    });

    // terrain deletion
    if app.is_mouse_pressed(input::mouse::Left) {
      time!(app.timers.deref(), "update.delete_terrain", || {
        for id in entities_in_front(app).into_iter() {
          if app.terrains.contains_key(&id) {
            remove_terrain(app, id);
          }
        }
      })
    }
  })
}

fn remove_terrain<'a>(app: &mut App<'a>, id: EntityId) {
  app.terrain_loader.push_back(Unload(id));
}

fn translate_mob(physics: &mut Physics<EntityId>, mob_buffers: &mut mob::MobBuffers, mob: &mut mob::Mob, delta_p: Vec3<GLfloat>) {
  if physics.translate(mob.id, delta_p).is_some() {
    mob.speed = mob.speed - delta_p;
  } else {
    let bounds = physics.get_bounds(mob.id).unwrap();
    mob_buffers.update(
      mob.id,
      to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
    );
  }
}

/// Returns ids of the closest entities in front of the cursor.
fn entities_in_front<'a>(app: &mut App<'a>) -> Vec<EntityId> {
  app.physics.octree.cast_ray(&app.player.forward_ray(), app.player.id)
}

fn load_terrain<'a>(app: &mut App<'a>, max: Option<uint>) {
  time!(app.timers.deref(), "load.terrain", || {
    // terrain loading
    let count = max.map_or(app.terrain_loader.len(), |x| cmp::min(x, app.terrain_loader.len()));
    for _ in range(0, count) {
      match app.terrain_loader.pop_front() {
        None => break,
        Some(op) => {
          let terrains = &mut app.terrains;
          let terrain_buffers = &mut app.terrain_buffers;
          let physics = &mut app.physics;
          match op {
            Load(id) => {
              let terrain = terrains.get(&id).unwrap();
              terrain_buffers.push(
                id,
                terrain,
              );
            },
            Unload(id) => {
              if terrains.remove(&id).is_some() {
                terrain_buffers.swap_remove(id);
                physics.remove(id);
              }
            },
          }
        }
      }
    }
  });
}

fn load_octree<'a>(app: &mut App<'a>) {
  time!(app.timers.deref(), "load.octree", || {
    // octree loading
    let count = cmp::min(OCTREE_LOAD_SPEED, app.octree_loader.deref().borrow().deref().len());
    for _ in range(0, count) {
      match app.octree_loader.borrow_mut().pop_front() {
        None => break,
        Some(Load((id, bounds))) => {
          app.octree_buffers.push(id, to_outlines(&bounds));
        },
        Some(Unload(id)) => {
          app.octree_buffers.swap_remove(id);
        }
      }
    }
  });
}
