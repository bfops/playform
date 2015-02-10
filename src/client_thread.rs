use client::Client;
use client_update::{ServerToClient, ViewToClient};
use server_update::ClientToServer;
use std::old_io::timer;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::time::duration::Duration;
use view_update::ClientToView;

pub fn client_thread(
  ups_from_server: Receiver<ServerToClient>,
  ups_to_server: Sender<ClientToServer>,
  ups_from_view: Receiver<ViewToClient>,
  ups_to_view: Sender<ClientToView>,
) {
  let mut client = Client::new();

  'client_loop:loop {
    'event_loop:loop {
      let update;
      match ups_from_view.try_recv() {
        Err(TryRecvError::Empty) => break 'event_loop,
        Err(e) => panic!("Error getting client local updates: {:?}", e),
        Ok(e) => update = e,
      };
      if !update.apply(&ups_to_server) {
        break 'client_loop;
      }
    }

    'event_loop:loop {
      let update;
      match ups_from_server.try_recv() {
        Err(TryRecvError::Empty) => break 'event_loop,
        Err(e) => panic!("Error getting client local updates: {:?}", e),
        Ok(e) => update = e,
      };
      update.apply(&mut client, &ups_to_view);
    }

    timer::sleep(Duration::milliseconds(0));
  }
}
