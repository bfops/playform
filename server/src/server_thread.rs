use common::communicate::{ClientToServer, ServerToClient};
use common::id_allocator::IdAllocator;
use common::interval_timer::IntervalTimer;
use common::process_events::process_channel;
use common::stopwatch::TimerSet;
use gaia_thread::gaia_thread;
use gaia_update::ServerToGaia;
use server::Server;
use server_update::{apply_client_to_server, apply_gaia_to_server};
use std::old_io::timer;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Mutex;
use std::thread::Thread;
use std::time::duration::Duration;
use time;
use update::update;

pub const UPDATES_PER_SECOND: u64 = 30;

pub fn server_thread(
  ups_from_client: &Receiver<ClientToServer>,
  ups_to_client: &Sender<ServerToClient>,
) {
  let timers = TimerSet::new();

  let id_allocator = Mutex::new(IdAllocator::new());
  let mut owner_allocator = IdAllocator::new();

  let mut world = Server::new(&ups_to_client, &mut owner_allocator, &timers);

  let (ups_to_gaia_send, ups_to_gaia_recv) = channel();
  let (ups_from_gaia_send, ups_from_gaia_recv) = channel();
  let _gaia_thread = {
    let terrain = world.terrain_game_loader.terrain.clone();
    Thread::spawn(move || {
      gaia_thread(
        &ups_to_gaia_recv,
        &ups_from_gaia_send,
        &id_allocator,
        terrain,
      );
    })
  };
  let ups_to_gaia = ups_to_gaia_send;
  let ups_from_gaia = ups_from_gaia_recv;

  let mut update_timer;
  {
    let now = time::precise_time_ns();
    let nanoseconds_per_second = 1000000000;
    update_timer = IntervalTimer::new(nanoseconds_per_second / UPDATES_PER_SECOND, now);
  }

  loop {
    let quit =
      !process_channel(
        ups_from_client,
        |update| apply_client_to_server(update, &mut world, &ups_to_client, &ups_to_gaia)
      );
    if quit {
      ups_to_gaia.send(ServerToGaia::Quit).unwrap();
      break;
    }

    process_channel(
      &ups_from_gaia,
      |update| {
        apply_gaia_to_server(update, &timers, &mut world, &ups_to_client, &ups_to_gaia);
        true
      },
    );

    let updates = update_timer.update(time::precise_time_ns());
    if updates > 0 {
      update(&timers, &mut world, &ups_to_client, &ups_to_gaia);
    }

    timer::sleep(Duration::milliseconds(0));
  }

  debug!("server exiting.");
}
