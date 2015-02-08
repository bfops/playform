use camera;
use color::{Color3, Color4};
use common::*;
use gl::types::*;
use light::{Light, set_point_light, set_ambient_light};
use mob;
use nalgebra::Vec3;
use opencl_context::CL;
use physics::Physics;
use view::View;
use world::World;
use std::ops::{Deref, DerefMut};
use stopwatch::TimerSet;
use terrain::terrain_block::BlockPosition;
use yaglw::gl_context::GLContext;

pub fn update(
  timers: &TimerSet,
  world: &mut World,
  view: &mut View,
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

      view.camera.translation = camera::translation(-world.player.position.to_vec());
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
              &mut view.gl,
              &mut world.physics,
              &mut view.mob_buffers,
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

    timers.time("update.sun", || {
      world.sun.update().map(|(rel_position, sun_color, ambient_light)| {
        set_point_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          &Light {
            position: world.player.position + rel_position,
            intensity: sun_color,
          }
        );

        set_ambient_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          Color3::of_rgb(
            sun_color.r * ambient_light,
            sun_color.g * ambient_light,
            sun_color.b * ambient_light,
          ),
        );

        view.gl.set_background_color(sun_color.r, sun_color.g, sun_color.b, 1.0);
      });
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
