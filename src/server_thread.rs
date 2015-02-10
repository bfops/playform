use client_update::ServerToClient;
use init::world;
use interval_timer::IntervalTimer;
use opencl_context::CL;
use server_update::ClientToServer;
use std::time::duration::Duration;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use stopwatch::TimerSet;
use time;
use update::update;

pub const UPDATES_PER_SECOND: u64 = 30;

pub fn server_thread(
  ups_from_client: Receiver<ClientToServer>,
  ups_to_client: Sender<ServerToClient>,
) {
  let timers = TimerSet::new();
  let cl = unsafe {
    CL::new()
  };

  let mut world = world::init(&cl, &ups_to_client, &timers);

  let mut update_timer;
  {
    let now = time::precise_time_ns();
    let nanoseconds_per_second = 1000000000;
    update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
  }

  'game_loop:loop {
    'event_loop:loop {
      let update;
      match ups_from_client.try_recv() {
        Err(TryRecvError::Empty) => break 'event_loop,
        Err(e) => panic!("Error getting world updates: {:?}", e),
        Ok(e) => update = e,
      };
      if !update.apply(&mut world) {
        break 'game_loop;
      }
    }

    let updates = update_timer.update(time::precise_time_ns());
    if updates > 0 {
      update(&timers, &mut world, &ups_to_client, &cl);
    }

    timer::sleep(Duration::milliseconds(0));
  }
}
