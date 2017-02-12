use cgmath::{Point3};
use collision::{Aabb3};
use rand;
use std::sync::Mutex;
use time;

use common::protocol;
use common::fnv_map;
use common::id_allocator;
use common::interval_timer::IntervalTimer;
use common::socket::SendSocket;

use entity;
use init_mobs::init_mobs;
use lod;
use mob;
use physics;
use player;
use sun::Sun;
use terrain_loader;

const UPDATES_PER_SECOND: u64 = 30;
const SUN_TICK_NS: u64 = 1600000;

pub struct Client {
  pub socket: SendSocket,
}

impl Client {
  pub fn send(&mut self, msg: protocol::ServerToClient) {
    use bincode::SizeLimit;
    use bincode::rustc_serialize::encode;
    let msg = encode(&msg, SizeLimit::Infinite).unwrap();
    match self.socket.write(msg.as_ref()) {
      Ok(()) => {},
      Err(err) => warn!("Error sending to client: {:?}", err),
    }
  }
}

// TODO: Audit for s/Mutex/RwLock.
pub struct T {
  pub players: Mutex<fnv_map::T<entity::id::Player, player::T>>,
  pub mobs: Mutex<fnv_map::T<entity::id::Misc, mob::Mob>>,

  pub id_allocator: Mutex<id_allocator::T<entity::id::T>>,
  pub owner_allocator: Mutex<id_allocator::T<lod::OwnerId>>,
  pub client_allocator: Mutex<id_allocator::T<protocol::ClientId>>,

  pub physics: Mutex<physics::T>,
  pub terrain_loader: terrain_loader::T,
  pub rng: Mutex<rand::StdRng>,

  pub clients: Mutex<fnv_map::T<protocol::ClientId, Client>>,

  pub sun: Mutex<Sun>,
  pub update_timer: Mutex<IntervalTimer>,
}

#[allow(missing_docs)]
pub fn new() -> T {
  let world_width: u32 = 1 << 11;
  let world_width = world_width as f32;
  let physics =
    physics::T::new(
      Aabb3::new(
        Point3 { x: -world_width, y: -512.0, z: -world_width },
        Point3 { x: world_width, y: 512.0, z: world_width },
      )
    );

  let id_allocator = id_allocator::new();
  let owner_allocator = Mutex::new(id_allocator::new());

  let server = T {
    players: Mutex::new(fnv_map::new()),
    mobs: Mutex::new(fnv_map::new()),

    id_allocator: Mutex::new(id_allocator),
    owner_allocator: owner_allocator,
    client_allocator: Mutex::new(id_allocator::new()),

    physics: Mutex::new(physics),
    terrain_loader: terrain_loader::T::new(),
    rng: {
      let seed = [0];
      let seed: &[usize] = &seed;
      Mutex::new(rand::SeedableRng::from_seed(seed))
    },

    clients: Mutex::new(fnv_map::new()),
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
