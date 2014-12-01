use camera;
use gl::types::*;
use nalgebra::Vec3;
use ncollide::bounding_volume::AABB;
use ncollide::ray::{Ray, Ray3};
use physics::Physics;
use state::EntityId;
use std::f32::consts::PI;

static MAX_JUMP_FUEL: uint = 4;
const MAX_STEP_HEIGHT: f32 = 1.0;

pub struct Player {
  pub camera: camera::Camera,
  // speed; units are world coordinates
  pub speed: Vec3<GLfloat>,
  // acceleration; units are world coordinates
  pub accel: Vec3<GLfloat>,
  // acceleration; x/z units are relative to player facing
  pub walk_accel: Vec3<GLfloat>,
  // this is depleted as we jump and replenished as we stand.
  pub jump_fuel: uint,
  // are we currently trying to jump? (e.g. holding the key).
  pub is_jumping: bool,
  pub id: EntityId,

  // rotation around the y-axis, in radians
  pub lateral_rotation: f32,
  // "pitch", in radians
  pub vertical_rotation: f32,
}

impl Player {
  /// Translates the player/camera by a vector.
  /// If the player collides with something with a small height jump, the player will shift upward.
  pub fn translate(&mut self, physics: &mut Physics<EntityId>, v: Vec3<GLfloat>) {
    let bounds = physics.bounds.get_mut(&self.id).unwrap();
    let init_bounds =
      AABB::new(
        *bounds.mins() + v,
        *bounds.maxs() + v,
      );
    let mut new_bounds = init_bounds.clone();
    // The height of the player's "step".
    let mut step = 0.0;
    let mut collided = false;
    loop {
      match Physics::reinsert(&mut physics.octree, self.id, bounds, new_bounds) {
        None => {
          break;
        },
        Some((collision_bounds, _)) => {
          collided = true;
          // Step to the top of whatever we hit.
          step = collision_bounds.maxs().y - init_bounds.mins().y;
          assert!(step > 0.0);

          if step > MAX_STEP_HEIGHT {
            // Step is too big; we just ran into something.
            break;
          }

          new_bounds =
            AABB::new(
              *init_bounds.mins() + Vec3::new(0.0, step, 0.0),
              *init_bounds.maxs() + Vec3::new(0.0, step, 0.0),
            );
        },
      }
    }

    self.camera.translate(v + Vec3::new(0.0, step, 0.0));

    if collided {
      if v.y < 0.0 {
        self.jump_fuel = MAX_JUMP_FUEL;
      }

      self.speed = self.speed - v;
    } else {
      if v.y < 0.0 {
        self.jump_fuel = 0;
      }
    }
  }

  pub fn update(&mut self, physics: &mut Physics<EntityId>) {
    if self.is_jumping {
      if self.jump_fuel > 0 {
        self.jump_fuel -= 1;
      } else {
        // this code is duplicated in a few places
        self.is_jumping = false;
        self.accel.y = self.accel.y - 0.3;
      }
    }

    let delta_p = self.speed;
    if delta_p.x != 0.0 {
      self.translate(physics, Vec3::new(delta_p.x, 0.0, 0.0));
    }
    if delta_p.y != 0.0 {
      self.translate(physics, Vec3::new(0.0, delta_p.y, 0.0));
    }
    if delta_p.z != 0.0 {
      self.translate(physics, Vec3::new(0.0, 0.0, delta_p.z));
    }

    let y_axis = Vec3::new(0.0, 1.0, 0.0);
    let walk_v =
        camera::from_axis_angle3(y_axis, self.lateral_rotation) * self.walk_accel;
    self.speed = self.speed + walk_v + self.accel;
    // friction
    self.speed = self.speed * Vec3::new(0.7, 0.99, 0.7 as f32);
  }

  /// Changes the camera's acceleration by the given `da`.
  pub fn walk(&mut self, da: Vec3<GLfloat>) {
    self.walk_accel = self.walk_accel + da * 0.2 as GLfloat;
  }

  /// Rotate the camera around the y axis, by `r` radians. Positive is
  /// counterclockwise.
  pub fn rotate_lateral(&mut self, r: GLfloat) {
    self.lateral_rotation = self.lateral_rotation + r;
    self.camera.rotate(Vec3::new(0.0, 1.0, 0.0), r);
  }

  /// Changes the camera pitch by `r` radians. Positive is up.
  /// Angles that "flip around" (i.e. looking too far up or down)
  /// are sliently rejected.
  pub fn rotate_vertical(&mut self, r: GLfloat) {
    let new_rotation = self.vertical_rotation + r;

    if new_rotation < -PI / 2.0
    || new_rotation >  PI / 2.0 {
      return
    }

    self.vertical_rotation = new_rotation;
    let axis = self.right();
    self.camera.rotate(axis, r);
  }

  // axes

  /// Return the "right" axis (i.e. the x-axis rotated to match you).
  pub fn right(&self) -> Vec3<GLfloat> {
    return
      camera::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation) *
      Vec3::new(1.0, 0.0, 0.0)
  }

  /// Return the "Ray axis (i.e. the z-axis rotated to match you).
  pub fn forward(&self) -> Vec3<GLfloat> {
    let y_axis = Vec3::new(0.0, 1.0, 0.0);
    let transform =
      camera::from_axis_angle3(self.right(), self.vertical_rotation) *
      camera::from_axis_angle3(y_axis, self.lateral_rotation);
    let forward_orig = Vec3::new(0.0, 0.0, -1.0);

    transform * forward_orig
  }

  pub fn forward_ray(&self) -> Ray3<f32> {
    Ray::new(self.camera.position, self.forward())
  }
}
