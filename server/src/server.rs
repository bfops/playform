use cgmath::{Aabb3, Point, Point3, Vector, Vector3};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::thread::JoinGuard;
use time;

use common::color::Color4;
use common::communicate::ServerToClient;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::interval_timer::IntervalTimer;
use common::lod::OwnerId;

use init_mobs::init_mobs;
use mob;
use physics::Physics;
use player::Player;
use sun::Sun;
use terrain::terrain;
use terrain::terrain_game_loader::TerrainGameLoader;

const UPDATES_PER_SECOND: u64 = 30;
const SUN_TICK_NS: u64 = 5000000;

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(1.0 / 2.0)
}

// TODO: Audit for s/Mutex/RwLock.
pub struct Server {
  pub player: Mutex<Player>,
  pub mobs: Mutex<HashMap<EntityId, mob::Mob>>,

  pub physics: Mutex<Physics>,
  pub id_allocator: Mutex<IdAllocator<EntityId>>,
  pub owner_allocator: Mutex<IdAllocator<OwnerId>>,
  pub terrain_game_loader: Mutex<TerrainGameLoader>,

  pub to_client: Mutex<Option<(Sender<Option<ServerToClient>>, JoinGuard<'static, ()>)>>,

  pub sun: Mutex<Sun>,
  pub update_timer: Mutex<IntervalTimer>,
}

impl Server {
  #[allow(missing_docs)]
  pub fn new() -> Server {
    let world_width: u32 = 1 << 11;
    let world_width = world_width as f32;
    let mut physics =
      Physics::new(
        Aabb3::new(
          Point3 { x: -world_width, y: -2.0 * terrain::AMPLITUDE as f32, z: -world_width },
          Point3 { x: world_width, y: 2.0 * terrain::AMPLITUDE as f32, z: world_width },
        )
      );

    let mut id_allocator = IdAllocator::new();
    let mut owner_allocator = Mutex::new(IdAllocator::new());

    let player = {
      let mut player = Player::new(id_allocator.allocate(), &mut owner_allocator);

      let min = Point3::new(0.0, terrain::AMPLITUDE as f32, 4.0);
      let max = min.add_v(&Vector3::new(1.0, 2.0, 1.0));
      let bounds = Aabb3::new(min, max);
      physics.insert_misc(player.entity_id, bounds.clone());

      player.position = center(&bounds);

      player.rotate_lateral(PI / 2.0);

      player
    };

    let server = Server {
      player: Mutex::new(player),
      mobs: Mutex::new(HashMap::new()),

      id_allocator: Mutex::new(id_allocator),
      physics: Mutex::new(physics),
      owner_allocator: owner_allocator,
      terrain_game_loader: Mutex::new(TerrainGameLoader::new()),

      to_client: Mutex::new(None),
      sun: Mutex::new(Sun::new(SUN_TICK_NS)),

      update_timer: {
        let now = time::precise_time_ns();
        let nanoseconds_per_second = 1000000000;
        Mutex::new(
          IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now)
        )
      }
    };

    init_mobs(&server);
    server
  }

  pub fn inform_client<UpdateClient>(&self, client: &mut UpdateClient)
    where UpdateClient: FnMut(ServerToClient)
  {
    let ids: Vec<EntityId> = self.mobs.lock().unwrap().iter().map(|(&x, _)| x).collect();
    for id in ids.into_iter() {
      let triangles = {
        let physics = self.physics.lock().unwrap();
        let bounds = physics.get_bounds(id).unwrap();
        mob::Mob::to_triangles(bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0))
        .iter()
        .map(|&x| x)
        .collect()
      };
      client(ServerToClient::AddMob(id, triangles));
    }
  }
}
