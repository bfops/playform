use color::Color4;
use common::*;
use gl::types::*;
use light::{Light, set_point_light};
use mob;
use nalgebra::Vec3;
use physics::Physics;
use state::App;
use std::f32::consts::PI;
use std::ops::{Deref, DerefMut};
use std::num::Float;
use terrain_block::BlockPosition;
use time;
use yaglw::gl_context::GLContext;

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
    app.timers.time("update.player", || {
      app.player.update(
        app.timers,
        app.gl_context,
        &mut app.terrain_game_loader,
        &mut app.id_allocator,
        &mut app.physics,
      );
    });

    app.timers.time("update.mobs", || {
      for (_, mob) in app.mobs.iter() {
        let mut mob_cell = mob.deref().borrow_mut();
        let mob = mob_cell.deref_mut();

        let block_position = BlockPosition::from_world_position(&mob.position);

        mob.solid_boundary.update(
          app.timers,
          app.gl_context,
          &mut app.terrain_game_loader,
          &mut app.id_allocator,
          &mut app.physics,
          block_position,
        );

        {
          let behavior = mob.behavior;
          (behavior)(app, mob);
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

    app.timers.time("update.sun", || {
      let ticks = app.sun_timer.update(time::precise_time_ns());
      app.sun += ticks as u16;

      let radius = 1024.0;
      // Convert the sun angle to radians.
      let sun_f = (app.sun as f32) * 2.0 * PI / 256.0;
      // position relative to player
      let sun_x = radius * sun_f.cos();
      let sun_y = radius * sun_f.sin();
      // "absolute" position
      let sun_position = app.player.camera.position + Vec3::new(sun_x, sun_y, 0.0);

      set_point_light(
        &mut app.terrain_shader,
        app.gl_context,
        &Light {
          position: sun_position,
          intensity: Vec3::new(0.6, 0.6, 0.2),
        }
      );
    });
  })
}

fn translate_mob(
  gl: &mut GLContext,
  physics: &mut Physics,
  mob_buffers: &mut mob::MobBuffers,
  mob: &mut mob::Mob,
  delta_p: Vec3<GLfloat>,
) {
  if physics.translate_misc(mob.id, delta_p).is_some() {
    mob.speed = mob.speed - delta_p;
  } else {
    let bounds = physics.get_bounds(mob.id).unwrap();
    mob.position = mob.position + delta_p;
    mob_buffers.update(
      gl,
      mob.id,
      &to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
    );
  }
}
