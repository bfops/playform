use common::entity::EntityId;
use common::matrix;
use nalgebra::{Pnt3, Vec3};
use ncollide_entities::bounding_volume::AABB;
use ncollide_queries::ray::{Ray, Ray3};
use physics::Physics;
use server::Server;
use std::f32::consts::PI;
use std::ops::DerefMut;
use std::sync::Mutex;

const MAX_JUMP_FUEL: u32 = 4;
const MAX_STEP_HEIGHT: f32 = 1.0;

pub struct Player {
  pub position: Pnt3<f32>,
  // speed; units are world coordinates
  pub speed: Vec3<f32>,
  // acceleration; units are world coordinates
  pub accel: Vec3<f32>,
  // acceleration; x/z units are relative to player facing
  pub walk_accel: Vec3<f32>,
  // this is depleted as we jump and replenished as we stand.
  pub jump_fuel: u32,
  // are we currently trying to jump? (e.g. holding the key).
  pub is_jumping: bool,
  pub entity_id: EntityId,

  // rotation around the y-axis, in radians
  pub lateral_rotation: f32,
  // "pitch", in radians
  pub vertical_rotation: f32,
}

impl Player {
  pub fn new(
    entity_id: EntityId,
  ) -> Player {
    Player {
      position: Pnt3::new(0.0, 0.0, 0.0),
      speed: Vec3::new(0.0, 0.0, 0.0),
      accel: Vec3::new(0.0, -0.1, 0.0),
      walk_accel: Vec3::new(0.0, 0.0, 0.0),
      jump_fuel: 0,
      is_jumping: false,
      entity_id: entity_id,
      lateral_rotation: 0.0,
      vertical_rotation: 0.0,
    }
  }

  /// Translates the player by a vector.
  /// If the player collides with something with a small height jump, the player will shift upward.
  /// Returns the actual amount moved by.
  pub fn translate(
    &mut self,
    physics: &Mutex<Physics>,
    v: Vec3<f32>,
  ) {
    let mut physics = physics.lock().unwrap();
    let physics = physics.deref_mut();
    let bounds = physics.bounds.get_mut(&self.entity_id).unwrap();
    let init_bounds =
      AABB::new(
        *bounds.mins() + v,
        *bounds.maxs() + v,
      );
    let mut new_bounds = init_bounds.clone();
    // The height of the player's "step".
    let mut step_height = 0.0;
    let mut collided = false;
    {
      loop {
        match physics.terrain_octree.intersect(&new_bounds, None) {
          None => {
            if Physics::reinsert(&mut physics.misc_octree, self.entity_id, bounds, new_bounds).is_some() {
              collided = true;
            } else {
              self.position = self.position + v + Vec3::new(0.0, step_height, 0.0);
            }
            break;
          },
          Some((collision_bounds, _)) => {
            collided = true;
            // Step to the top of whatever we hit.
            step_height = collision_bounds.maxs().y - init_bounds.mins().y;
            assert!(step_height > 0.0);

            if step_height > MAX_STEP_HEIGHT {
              // Step is too big; we just ran into something.
              break;
            }

            new_bounds =
              AABB::new(
                *init_bounds.mins() + Vec3::new(0.0, step_height, 0.0),
                *init_bounds.maxs() + Vec3::new(0.0, step_height, 0.0),
              );
          },
        }
      }
    }

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

  pub fn update(
    &mut self,
    server: &Server,
  ) {
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
      self.translate(&server.physics, Vec3::new(delta_p.x, 0.0, 0.0));
    }
    if delta_p.y != 0.0 {
      self.translate(&server.physics, Vec3::new(0.0, delta_p.y, 0.0));
    }
    if delta_p.z != 0.0 {
      self.translate(&server.physics, Vec3::new(0.0, 0.0, delta_p.z));
    }

    let y_axis = Vec3::new(0.0, 1.0, 0.0);
    let walk_v =
        matrix::from_axis_angle3(y_axis, self.lateral_rotation) * self.walk_accel;
    self.speed = self.speed + walk_v + self.accel;
    // friction
    self.speed = self.speed * Vec3::new(0.7, 0.99, 0.7 as f32);
  }

  /// Changes the player's acceleration by the given `da`.
  pub fn walk(&mut self, da: Vec3<f32>) {
    self.walk_accel = self.walk_accel + da * 0.2 as f32;
  }

  /// Rotate the player around the y axis, by `r` radians. Positive is counterclockwise.
  pub fn rotate_lateral(&mut self, r: f32) {
    self.lateral_rotation = self.lateral_rotation + r;
  }

  /// Changes the player's pitch by `r` radians. Positive is up.
  /// Angles that "flip around" (i.e. looking too far up or down)
  /// are sliently rejected.
  pub fn rotate_vertical(&mut self, r: f32) {
    let new_rotation = self.vertical_rotation + r;

    if new_rotation < -PI / 2.0
    || new_rotation >  PI / 2.0 {
      return
    }

    self.vertical_rotation = new_rotation;
  }

  // axes

  /// Return the "right" axis (i.e. the x-axis rotated to match you).
  pub fn right(&self) -> Vec3<f32> {
    return
      matrix::from_axis_angle3(Vec3::new(0.0, 1.0, 0.0), self.lateral_rotation) *
      Vec3::new(1.0, 0.0, 0.0)
  }

  /// Return the "Ray axis (i.e. the z-axis rotated to match you).
  pub fn forward(&self) -> Vec3<f32> {
    let y_axis = Vec3::new(0.0, 1.0, 0.0);
    let transform =
      matrix::from_axis_angle3(self.right(), self.vertical_rotation) *
      matrix::from_axis_angle3(y_axis, self.lateral_rotation);
    let forward_orig = Vec3::new(0.0, 0.0, -1.0);

    transform * forward_orig
  }

  #[allow(dead_code)]
  pub fn forward_ray(&self) -> Ray3<f32> {
    Ray::new(self.position, self.forward())
  }
}
