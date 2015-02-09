use init::world;
use interval_timer::IntervalTimer;
use nalgebra::{Vec2, Vec3};
use opencl_context::CL;
use std::time::duration::Duration;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use stopwatch::TimerSet;
use time;
use update::update;
use view_thread::ViewUpdate;

pub const UPDATES_PER_SECOND: u64 = 30;

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
