use common::color::Color4;
use common::communicate::ServerToClient;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::OwnerId;
use common::stopwatch::TimerSet;
use init_mobs::init_mobs;
use mob;
use nalgebra::{Pnt3, Vec3};
use ncollide_entities::bounding_volume::{AABB, AABB3};
use physics::Physics;
use player::Player;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use sun::Sun;
use terrain::terrain;
use terrain::terrain_game_loader::TerrainGameLoader;

const SUN_TICK_NS: u64 = 5000000;

fn center(bounds: &AABB3<f32>) -> Pnt3<f32> {
  (*bounds.mins() + bounds.maxs().to_vec()) / (2.0 as f32)
}

pub struct Server<'a> {
  pub physics: Physics,
  pub player: Player<'a>,
  pub mobs: HashMap<EntityId, Rc<RefCell<mob::Mob<'a>>>>,
  pub sun: Sun,

  pub id_allocator: Arc<Mutex<IdAllocator<EntityId>>>,
  pub owner_allocator: IdAllocator<OwnerId>,
  pub terrain_game_loader: TerrainGameLoader,

  pub to_client: Option<Sender<ServerToClient>>,
}

impl<'a> Server<'a> {
  #[allow(missing_docs)]
  pub fn new<'b:'a>(
    timers: &TimerSet,
  ) -> Server<'b> {
    let terrain_game_loader = TerrainGameLoader::new();

    let world_width: u32 = 1 << 11;
    let world_width = world_width as f32;
    let mut physics =
      Physics::new(
        AABB::new(
          Pnt3 { x: -world_width, y: -2.0 * terrain::AMPLITUDE as f32, z: -world_width },
          Pnt3 { x: world_width, y: 2.0 * terrain::AMPLITUDE as f32, z: world_width },
        )
      );

    let id_allocator = Arc::new(Mutex::new(IdAllocator::new()));
    let mut owner_allocator = IdAllocator::new();

    let mobs =
      timers.time("init_mobs", || {
        init_mobs(
          &mut physics,
          id_allocator.deref(),
          &mut owner_allocator,
        )
      });

    let player = {
      let mut player = Player::new(
        id_allocator.lock().unwrap().allocate(),
        &mut owner_allocator,
      );

      let min = Pnt3::new(0.0, terrain::AMPLITUDE as f32, 4.0);
      let max = min + Vec3::new(1.0, 2.0, 1.0);
      let bounds = AABB::new(min, max);
      physics.insert_misc(player.id, bounds.clone());

      player.position = center(&bounds);

      player.rotate_lateral(PI / 2.0);

      player
    };

    Server {
      physics: physics,
      player: player,
      mobs: mobs,
      sun: Sun::new(SUN_TICK_NS),

      id_allocator: id_allocator,
      owner_allocator: owner_allocator,
      terrain_game_loader: terrain_game_loader,

      to_client: None,
    }
  }

  pub fn inform_client(&self, client: &Sender<ServerToClient>) {
    for (&id, _) in self.mobs.iter() {
      let bounds = self.physics.get_bounds(id).unwrap();
      let triangles =
        mob::Mob::to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
        .iter()
        .map(|&x| x)
        .collect();
      client.send(ServerToClient::AddMob(id, triangles)).unwrap();
    }
  }

  #[inline]
  pub fn get_bounds(&self, id: EntityId) -> &AABB3<f32> {
    self.physics.get_bounds(id).unwrap()
  }
}
