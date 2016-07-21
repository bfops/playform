use cgmath;
use cgmath::{Aabb3, Point, Point3, Matrix, Matrix3, Ray, Ray3, Vector, Vector3};
use std::f32::consts::PI;
use std::ops::DerefMut;
use std::sync::Mutex;
use stopwatch;

use common::entity_id;
use common::id_allocator;
use common::surroundings_loader;
use common::voxel;

use lod;
use physics::Physics;
use server;
use update_gaia;
use update_world::load_placeholders;

const MAX_JUMP_FUEL: u32 = 4;
const MAX_STEP_HEIGHT: f32 = 1.0;

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
pub enum Collision {
  Terrain(entity_id::T),
  Misc(entity_id::T),
}

// TODO: Add ObservablePlayer struct as a subset.
pub struct T {
  pub position: Point3<f32>,
  // speed; units are world coordinates
  pub speed: Vector3<f32>,
  // acceleration; units are world coordinates
  pub accel: Vector3<f32>,
  // acceleration; x/z units are relative to player facing
  pub walk_accel: Vector3<f32>,
  // this is depleted as we jump and replenished as we stand.
  pub jump_fuel: u32,
  // are we currently trying to jump? (e.g. holding the key).
  pub is_jumping: bool,
  pub entity_id: entity_id::T,

  // rotation around the y-axis, in radians
  pub lateral_rotation: f32,
  // "pitch", in radians
  pub vertical_rotation: f32,

  surroundings_loader: surroundings_loader::T,
  surroundings_owner: lod::OwnerId,
  // Nearby blocks should be made solid if they aren't loaded yet.
  solid_boundary: surroundings_loader::T,
  solid_owner: lod::OwnerId,
}

impl T {
  pub fn new(
    entity_id: entity_id::T,
    owner_allocator: &Mutex<id_allocator::T<lod::OwnerId>>,
  ) -> T {
    let surroundings_owner = id_allocator::allocate(owner_allocator);
    let solid_owner = id_allocator::allocate(owner_allocator);
    T {
      position            : Point3::new(0.0, 0.0, 0.0),
      speed               : Vector3::new(0.0, 0.0, 0.0),
      accel               : Vector3::new(0.0, -0.1, 0.0),
      walk_accel          : Vector3::new(0.0, 0.0, 0.0),
      jump_fuel           : 0,
      is_jumping          : false,
      entity_id           : entity_id,
      lateral_rotation    : 0.0,
      vertical_rotation   : 0.0,

      surroundings_loader : surroundings_loader::new(8, Vec::new()),
      solid_boundary      : surroundings_loader::new(8, Vec::new()),
      surroundings_owner  : surroundings_owner,
      solid_owner         : solid_owner,
    }
  }

  /// Translates the player by a vector.
  /// If the player collides with something with a small height jump, the player will shift upward.
  /// Returns the actual amount moved by.
  pub fn translate(
    &mut self,
    physics: &Mutex<Physics>,
    v: Vector3<f32>,
  ) -> (Aabb3<f32>, Vec<Collision>)
  {
    let mut physics = physics.lock().unwrap();
    let physics = physics.deref_mut();
    let bounds = physics.bounds.get_mut(&self.entity_id).unwrap();
    let init_bounds =
      Aabb3::new(
        bounds.min.add_v(&v),
        bounds.max.add_v(&v),
      );

    let mut new_bounds = init_bounds;
    let mut collisions = Vec::new();
    let mut collided = false;
    // The height of the player's "step".
    let mut step_height = 0.0;
    loop {
      match physics.terrain_octree.intersect(&new_bounds, None) {
        None => {
          if let Some((_, id)) = Physics::reinsert(&mut physics.misc_octree, self.entity_id, bounds, &new_bounds) {
            collided = true;
            collisions.push(Collision::Misc(id));
          } else {
            self.position.add_self_v(&v);
            self.position.add_self_v(&Vector3::new(0.0, step_height, 0.0));
          }
          break
        },
        Some((collision_bounds, id)) => {
          collisions.push(Collision::Terrain(id));
          collided = true;

          // Step to the top of whatever we hit.
          step_height = collision_bounds.max.y - init_bounds.min.y;
          assert!(step_height > 0.0);

          if step_height > MAX_STEP_HEIGHT {
            // Step is too big; we just ran into something.
            break
          }

          new_bounds =
            Aabb3::new(
              init_bounds.min.add_v(&Vector3::new(0.0, step_height, 0.0)),
              init_bounds.max.add_v(&Vector3::new(0.0, step_height, 0.0)),
            );
        },
      }
    }

    if collided {
      if v.y < 0.0 {
        self.jump_fuel = MAX_JUMP_FUEL;
      }

      self.speed.add_self_v(&-v);
    } else {
      if v.y < 0.0 {
        self.jump_fuel = 0;
      }
    }

    (*bounds, collisions)
  }

  pub fn update<RequestBlock>(
    &mut self,
    server: &server::T,
    request_block: &mut RequestBlock,
  ) -> (Aabb3<f32>, Vec<Collision>) where
    RequestBlock: FnMut(update_gaia::Message),
  {
    let player_position =
      Point3::new(
        self.position.x as i32,
        self.position.y as i32,
        self.position.z as i32,
      );

    stopwatch::time("update.player.surroundings", || {
      let owner = self.surroundings_owner;
      for (pos, load_type) in self.surroundings_loader.updates(&player_position) {
        let pos = voxel::bounds::new(pos.x, pos.y, pos.z, 0);
        match load_type {
          surroundings_loader::LoadType::Load | surroundings_loader::LoadType::Update => {
            server.terrain_loader.load(
              &server.id_allocator,
              &server.physics,
              &pos,
              lod::Full,
              owner,
              request_block,
            );
          },
          surroundings_loader::LoadType::Unload => {
            server.terrain_loader.unload(
              &server.physics,
              &pos,
              owner,
            );
          },
        }
      }

      let owner = self.solid_owner;
      for (pos, load_type) in self.solid_boundary.updates(&player_position) {
        let block_position = voxel::bounds::new(pos.x, pos.y, pos.z, 0);
        load_placeholders(
          owner,
          server,
          request_block,
          &block_position,
          load_type,
        )
      }
    });

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
    let mut new_bounds = *server.physics.lock().unwrap().bounds.get_mut(&self.entity_id).unwrap();
    let mut collisions = Vec::new();
    if delta_p.x != 0.0 {
      let (b, c) = self.translate(&server.physics, Vector3::new(delta_p.x, 0.0, 0.0));
      new_bounds = b;
      collisions.extend_from_slice(c.as_slice());
    }
    if delta_p.y != 0.0 {
      let (b, c) = self.translate(&server.physics, Vector3::new(0.0, delta_p.y, 0.0));
      new_bounds = b;
      collisions.extend_from_slice(c.as_slice());
    }
    if delta_p.z != 0.0 {
      let (b, c) = self.translate(&server.physics, Vector3::new(0.0, 0.0, delta_p.z));
      new_bounds = b;
      collisions.extend_from_slice(c.as_slice());
    }

    let y_axis = Vector3::new(0.0, 1.0, 0.0);
    let walk_v =
        Matrix3::from_axis_angle(&y_axis, cgmath::rad(self.lateral_rotation))
        .mul_v(&self.walk_accel);
    self.speed.add_self_v(&walk_v);
    self.speed.add_self_v(&self.accel);
    // friction
    self.speed.mul_self_v(&Vector3::new(0.7, 0.99, 0.7 as f32));

    (new_bounds, collisions)
  }

  /// Changes the player's acceleration by the given `da`.
  pub fn walk(&mut self, da: Vector3<f32>) {
    self.walk_accel.add_self_v(&da.mul_s(0.1));
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
  pub fn right(&self) -> Vector3<f32> {
    Matrix3::from_axis_angle(&Vector3::new(0.0, 1.0, 0.0), cgmath::rad(self.lateral_rotation)).mul_v(&Vector3::new(1.0, 0.0, 0.0))
  }

  /// Return the "Ray axis (i.e. the z-axis rotated to match you).
  pub fn forward(&self) -> Vector3<f32> {
    let y_axis = Vector3::new(0.0, 1.0, 0.0);
    let transform =
      Matrix3::from_axis_angle(&self.right(), cgmath::rad(self.vertical_rotation))
      .mul_m(&Matrix3::from_axis_angle(&y_axis, cgmath::rad(self.lateral_rotation)));
    let forward_orig = Vector3::new(0.0, 0.0, -1.0);

    transform.mul_v(&forward_orig)
  }

  pub fn forward_ray(&self) -> Ray3<f32> {
    Ray::new(self.position, self.forward())
  }
}
