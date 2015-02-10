use log;
use logger::Logger;
use std::sync::mpsc::channel;
use std::thread::Thread;
use view_thread::view_thread;
use world_thread::world_thread;

pub fn main() {
  log::set_logger(|max_log_level| {
    max_log_level.set(log::LogLevelFilter::Debug);
    Box::new(Logger)
  }).unwrap();

  debug!("starting");

  let (world_updates_send, world_updates_recv) = channel();
  let (view_send, view_recv) = channel();

  let _world_thread = Thread::spawn(|| world_thread(world_updates_recv, view_send));

  view_thread(world_updates_send, view_recv);

  debug!("finished");
}
