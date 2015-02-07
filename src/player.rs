use camera;
use gl::types::*;
use id_allocator::IdAllocator;
use lod_map::{LOD, OwnerId};
use nalgebra::{Pnt3, Vec3};
use ncollide_entities::bounding_volume::{AABB, AABB3};
use ncollide_queries::ray::{Ray, Ray3};
use opencl_context::CL;
use physics::Physics;
use renderer::Renderer;
use state::EntityId;
use std::f32::consts::PI;
use std::iter::range_inclusive;
use std::num;
use stopwatch::TimerSet;
use surroundings_loader::SurroundingsLoader;
use terrain::terrain;
use terrain::terrain_block::BlockPosition;
use terrain::terrain_game_loader::TerrainGameLoader;

const LOD_THRESHOLDS: [i32; 3] = [1, 8, 32];

const MAX_JUMP_FUEL: u32 = 4;
const MAX_STEP_HEIGHT: f32 = 1.0;

fn center(bounds: &AABB3<f32>) -> Pnt3<GLfloat> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as GLfloat)
}

pub struct Player<'a> {
  pub camera: camera::Camera,
  // speed; units are world coordinates
  pub speed: Vec3<GLfloat>,
  // acceleration; units are world coordinates
  pub accel: Vec3<GLfloat>,
  // acceleration; x/z units are relative to player facing
  pub walk_accel: Vec3<GLfloat>,
  // this is depleted as we jump and replenished as we stand.
  pub jump_fuel: u32,
  // are we currently trying to jump? (e.g. holding the key).
  pub is_jumping: bool,
  pub id: EntityId,

  // rotation around the y-axis, in radians
  pub lateral_rotation: f32,
  // "pitch", in radians
  pub vertical_rotation: f32,

  pub surroundings_loader: SurroundingsLoader<'a>,
  // Nearby blocks should be made solid if they aren't loaded yet.
  pub solid_boundary: SurroundingsLoader<'a>,
}

impl<'a> Player<'a> {
  pub fn new(
    id_allocator: &mut IdAllocator<EntityId>,
    owner_allocator: &mut IdAllocator<OwnerId>,
    physics: &mut Physics,
    load_distance: i32,
  ) -> Player<'a> {
    let mut player = Player {
      camera: camera::Camera::unit(),
      speed: Vec3::new(0.0, 0.0, 0.0),
      accel: Vec3::new(0.0, -0.1, 0.0),
      walk_accel: Vec3::new(0.0, 0.0, 0.0),
      jump_fuel: 0,
      is_jumping: false,
      id: id_allocator.allocate(),
      lateral_rotation: 0.0,
      vertical_rotation: 0.0,
      surroundings_loader:
        SurroundingsLoader::new(
          owner_allocator.allocate(),
          load_distance,
          Box::new(|d| LOD::LodIndex(Player::lod_index(d))),
        ),
      solid_boundary:
        SurroundingsLoader::new(
          owner_allocator.allocate(),
          1,
          Box::new(|_| LOD::Placeholder),
        ),
    };

    let min = Pnt3::new(0.0, terrain::AMPLITUDE as f32, 4.0);
    let max = min + Vec3::new(1.0, 2.0, 1.0);
    let bounds = AABB::new(min, max);
    physics.insert_misc(player.id, bounds.clone());

    // initialize the projection matrix
    player.camera.translate(center(&bounds).to_vec());
    player.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 2048.0);
    player.rotate_lateral(PI / 2.0);

    player
  }

  /// Translates the player/camera by a vector.
  /// If the player collides with something with a small height jump, the player will shift upward.
  pub fn translate(&mut self, physics: &mut Physics, v: Vec3<GLfloat>) {
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
      match physics.terrain_octree.intersect(&new_bounds, None) {
        None => {
          let move_successful = Physics::reinsert(&mut physics.misc_octree, self.id, bounds, new_bounds).is_none();
          if move_successful {
            self.camera.translate(v + Vec3::new(0.0, step, 0.0));
          }
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
    timers: &TimerSet,
    renderer: &mut Renderer,
    cl: &CL,
    terrain_game_loader: &mut TerrainGameLoader,
    id_allocator: &mut IdAllocator<EntityId>,
    physics: &mut Physics,
  ) {
    let block_position = BlockPosition::from_world_position(&self.camera.position);

    timers.time("update.player.surroundings", || {
      self.surroundings_loader.update(
        timers,
        renderer,
        cl,
        terrain_game_loader,
        id_allocator,
        physics,
        block_position,
      );

      self.solid_boundary.update(
        timers,
        renderer,
        cl,
        terrain_game_loader,
        id_allocator,
        physics,
        block_position,
      );
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

  #[allow(dead_code)]
  pub fn forward_ray(&self) -> Ray3<f32> {
    Ray::new(self.camera.position, self.forward())
  }

  pub fn lod_index(distance: i32) -> u32 {
    assert!(distance >= 0);
    let mut lod = 0;
    while
      lod < LOD_THRESHOLDS.len()
      && LOD_THRESHOLDS[lod] < distance
    {
      lod += 1;
    }
    num::cast(lod).unwrap()
  }

  pub fn load_distance(mut polygon_budget: i32) -> i32 {
    // TODO: This should try to account for VRAM not used on a per-poly basis.

    let mut load_distance = 0;
    let mut prev_threshold = 0;
    let mut prev_square = 0;
    for (&threshold, &quality) in
      LOD_THRESHOLDS.iter()
        .zip(terrain::LOD_QUALITY.iter()) {
      let polygons_per_block = (quality * quality * 4) as i32;
      for i in range_inclusive(prev_threshold, threshold) {
        let i = 2 * i + 1;
        let square = i * i;
        let polygons_in_layer = (square - prev_square) * polygons_per_block;
        polygon_budget -= polygons_in_layer;
        if polygon_budget < 0 {
          break;
        }

        load_distance += 1;
        prev_square = square;
      }
      prev_threshold = threshold + 1;
    }

    let mut width = 2 * prev_threshold + 1;
    loop {
      let square = width * width;
      // The "to infinity and beyond" quality.
      let quality = terrain::LOD_QUALITY[LOD_THRESHOLDS.len()];
      let polygons_per_block = (quality * quality * 4) as i32;
      let polygons_in_layer = (square - prev_square) * polygons_per_block;
      polygon_budget -= polygons_in_layer;

      if polygon_budget < 0 {
        break;
      }

      width += 2;
      load_distance += 1;
      prev_square = square;
    }

    load_distance
  }
}
