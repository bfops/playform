use cgmath;
use cgmath::{Aabb3, Point, Point3, Matrix, Matrix3, Ray, Ray3, Vector, Vector3};
use std::f32::consts::PI;
use std::ops::DerefMut;
use std::sync::Mutex;
use stopwatch;

use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::{LOD, LODIndex, OwnerId};
use common::surroundings_loader::{SurroundingsLoader, LoadType};

use physics::Physics;
use server::Server;
use update_gaia::ServerToGaia;
use update_world::load_placeholders;

const MAX_JUMP_FUEL: u32 = 4;
const MAX_STEP_HEIGHT: f32 = 1.0;

// TODO: Add ObservablePlayer struct as a subset.
pub struct Player {
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
  pub entity_id: EntityId,

  // rotation around the y-axis, in radians
  pub lateral_rotation: f32,
  // "pitch", in radians
  pub vertical_rotation: f32,

  surroundings_loader: SurroundingsLoader,
  surroundings_owner: OwnerId,
  // Nearby blocks should be made solid if they aren't loaded yet.
  solid_boundary: SurroundingsLoader,
  solid_owner: OwnerId,
}

impl Player {
  pub fn new(
    entity_id: EntityId,
    owner_allocator: &Mutex<IdAllocator<OwnerId>>,
  ) -> Player {
    let surroundings_owner = owner_allocator.lock().unwrap().allocate();
    let solid_owner = owner_allocator.lock().unwrap().allocate();
    Player {
      position: Point3::new(0.0, 0.0, 0.0),
      speed: Vector3::new(0.0, 0.0, 0.0),
      accel: Vector3::new(0.0, -0.1, 0.0),
      walk_accel: Vector3::new(0.0, 0.0, 0.0),
      jump_fuel: 0,
      is_jumping: false,
      entity_id: entity_id,
      lateral_rotation: 0.0,
      vertical_rotation: 0.0,

      surroundings_loader: SurroundingsLoader::new(1, Vec::new()),
      solid_boundary:  SurroundingsLoader::new(1, Vec::new()),
      surroundings_owner:  surroundings_owner,
      solid_owner: solid_owner,
    }
  }

  /// Translates the player by a vector.
  /// If the player collides with something with a small height jump, the player will shift upward.
  /// Returns the actual amount moved by.
  pub fn translate(
    &mut self,
    physics: &Mutex<Physics>,
    v: Vector3<f32>,
  ) {
    let mut physics = physics.lock().unwrap();
    let physics = physics.deref_mut();
    let bounds = physics.bounds.get_mut(&self.entity_id).unwrap();
    let init_bounds =
      Aabb3::new(
        bounds.min.add_v(&v),
        bounds.max.add_v(&v),
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
              self.position.add_self_v(&v);
              self.position.add_self_v(&Vector3::new(0.0, step_height, 0.0));
            }
            break;
          },
          Some((collision_bounds, _)) => {
            collided = true;
            // Step to the top of whatever we hit.
            step_height = collision_bounds.max.y - init_bounds.min.y;
            assert!(step_height > 0.0);

            if step_height > MAX_STEP_HEIGHT {
              // Step is too big; we just ran into something.
              break;
            }

            new_bounds =
              Aabb3::new(
                init_bounds.min.add_v(&Vector3::new(0.0, step_height, 0.0)),
                init_bounds.max.add_v(&Vector3::new(0.0, step_height, 0.0)),
              );
          },
        }
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
  }

  pub fn update<RequestBlock>(
    &mut self,
    server: &Server,
    request_block: &mut RequestBlock,
  ) where
    RequestBlock: FnMut(ServerToGaia),
  {
    let block_position = BlockPosition::from_world_position(&self.position);

    stopwatch::time("update.player.surroundings", || {
      let owner = self.surroundings_owner;
      for (pos, load_type) in self.surroundings_loader.updates(block_position) {
        match load_type {
          LoadType::Load | LoadType::Update => {
            server.terrain_loader.lock().unwrap().load(
              &server.id_allocator,
              &server.physics,
              &pos,
              LOD::LodIndex(LODIndex(0)),
              owner,
              request_block,
            );
          },
          LoadType::Unload => {
            server.terrain_loader.lock().unwrap().unload(
              &server.physics,
              &pos,
              owner,
            );
          },
        }
      }

      let owner = self.solid_owner;
      for (block_position, load_type) in self.solid_boundary.updates(block_position) {
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
    if delta_p.x != 0.0 {
      self.translate(&server.physics, Vector3::new(delta_p.x, 0.0, 0.0));
    }
    if delta_p.y != 0.0 {
      self.translate(&server.physics, Vector3::new(0.0, delta_p.y, 0.0));
    }
    if delta_p.z != 0.0 {
      self.translate(&server.physics, Vector3::new(0.0, 0.0, delta_p.z));
    }

    let y_axis = Vector3::new(0.0, 1.0, 0.0);
    let walk_v =
        Matrix3::from_axis_angle(&y_axis, cgmath::rad(self.lateral_rotation))
        .mul_v(&self.walk_accel);
    self.speed.add_self_v(&walk_v);
    self.speed.add_self_v(&self.accel);
    // friction
    self.speed.mul_self_v(&Vector3::new(0.7, 0.99, 0.7 as f32));
  }

  /// Changes the player's acceleration by the given `da`.
  pub fn walk(&mut self, da: Vector3<f32>) {
    self.walk_accel.add_self_v(&da.mul_s(0.2));
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
