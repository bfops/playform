//! Common code for event processing.

use std::sync::mpsc::{Receiver, TryRecvError};

/// Process all available events from a channel.
pub fn process_channel<T, Apply>(chan: &Receiver<T>, mut apply: Apply) -> bool
  where
    T: Send,
    Apply: FnMut(T) -> bool,
{
  loop {
    match chan.try_recv() {
      Err(TryRecvError::Empty) => break,
      Err(e) => panic!("Error receiving from channel: {:?}", e),
      Ok(update) => {
        if !apply(update) {
          return false
        }
      },
    }
  }

  true
}
