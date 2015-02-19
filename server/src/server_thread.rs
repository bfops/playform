use common::communicate::ClientToServer;
use common::interval_timer::IntervalTimer;
use common::process_events::process_channel;
use common::stopwatch::TimerSet;
use gaia_update::ServerToGaia;
use nanomsg::Endpoint;
use server::Server;
use server_update::{GaiaToServer, apply_client_to_server, apply_gaia_to_server};
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver};
use std::time::duration::Duration;
use time;
use update::update;

pub const UPDATES_PER_SECOND: u64 = 30;

pub fn server_thread(
  timers: &TimerSet,
  mut world: Server,
  client_endpoints: &mut Vec<Endpoint>,
  ups_from_client: &Receiver<ClientToServer>,
  ups_from_gaia: &Receiver<GaiaToServer>,
  ups_to_gaia: &Sender<ServerToGaia>,
) {
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
        |update|
          apply_client_to_server(
            timers,
            update,
            &mut world,
            client_endpoints,
            ups_to_gaia,
          )
      );
    if quit {
      ups_to_gaia.send(ServerToGaia::Quit).unwrap();
      break;
    }

    process_channel(
      &ups_from_gaia,
      |update| {
        apply_gaia_to_server(update, timers, &mut world, &ups_to_gaia);
        true
      },
    );

    let updates = update_timer.update(time::precise_time_ns());
    if updates > 0 {
      update(timers, &mut world, &ups_to_gaia);
    }

    timer::sleep(Duration::milliseconds(0));
  }

  debug!("server exiting.");
}
