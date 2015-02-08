use id_allocator::IdAllocator;
use init::world;
use interval_timer::IntervalTimer;
use mob;
use nalgebra::{Vec2, Vec3};
use ncollide_entities::bounding_volume::AABB3;
use opencl_context::CL;
use physics::Physics;
use player::Player;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::ops::Add;
use std::rc::Rc;
use std::time::duration::Duration;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use stopwatch::TimerSet;
use sun::Sun;
use terrain::terrain_game_loader::TerrainGameLoader;
use time;
use update::update;
use view::ViewUpdate;

pub const UPDATES_PER_SECOND: u64 = 30;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityId(u32);

impl Default for EntityId {
  fn default() -> EntityId {
    EntityId(0)
  }
}

impl Add<u32> for EntityId {
  type Output = EntityId;

  fn add(self, rhs: u32) -> EntityId {
    let EntityId(i) = self;
    EntityId(i + rhs)
  }
}

pub struct World<'a> {
  pub physics: Physics,
  pub player: Player<'a>,
  pub mobs: HashMap<EntityId, Rc<RefCell<mob::Mob<'a>>>>,
  pub sun: Sun,

  pub id_allocator: IdAllocator<EntityId>,
  pub terrain_game_loader: TerrainGameLoader,
}

impl<'a> World<'a> {
  #[inline]
  pub fn get_bounds(&self, id: EntityId) -> &AABB3<f32> {
    self.physics.get_bounds(id).unwrap()
  }
}

#[derive(Debug, Clone)]
pub enum WorldUpdate {
  Walk(Vec3<f32>),
  RotatePlayer(Vec2<f32>),
  StartJump,
  StopJump,
  Quit,
}

unsafe impl Send for WorldUpdate {}

pub fn world_thread(
  world_updates: Receiver<WorldUpdate>,
  view: Sender<ViewUpdate>,
) {
  let timers = TimerSet::new();
  let cl = unsafe {
    CL::new()
  };

  let mut world = world::init(&cl, &view, &timers);

  let mut update_timer;
  {
    let now = time::precise_time_ns();
    let nanoseconds_per_second = 1000000000;
    update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
  }

  'game_loop:loop {
    'event_loop:loop {
      let event;
      match world_updates.try_recv() {
        Err(TryRecvError::Empty) => break 'event_loop,
        Err(e) => panic!("Error getting world updates: {:?}", e),
        Ok(e) => event = e,
      };
      match event {
        WorldUpdate::Quit => {
          break 'game_loop;
        },
        WorldUpdate::StartJump => {
          if !world.player.is_jumping {
            world.player.is_jumping = true;
            // this 0.3 is duplicated in a few places
            world.player.accel.y = world.player.accel.y + 0.3;
          }
        },
        WorldUpdate::StopJump => {
          if world.player.is_jumping {
            world.player.is_jumping = false;
            // this 0.3 is duplicated in a few places
            world.player.accel.y = world.player.accel.y - 0.3;
          }
        },
        WorldUpdate::Walk(v) => {
          world.player.walk(v);
        },
        WorldUpdate::RotatePlayer(v) => {
          world.player.rotate_lateral(v.x);
          world.player.rotate_vertical(v.y);
        },
      }
    }

    let updates = update_timer.update(time::precise_time_ns());
    if updates > 0 {
      update(&timers, &mut world, &view, &cl);
    }

    timer::sleep(Duration::milliseconds(0));
  }
}
