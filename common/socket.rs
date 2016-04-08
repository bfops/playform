//! One-way socket wrapper data structures.

use nanomsg::{Endpoint, Socket, Protocol, Error};
use std;
use std::convert::AsRef;
use std::io::{Read, Write};
use std::time::Duration;

/// A send-only socket.
pub struct SendSocket {
  socket: Socket,
  endpoint: Endpoint,
}

fn as_millis(duration: Duration) -> isize {
  (duration.as_secs() * 1_000) as isize + (duration.subsec_nanos() / 1_000_000) as isize
}

impl SendSocket {
  #[allow(missing_docs)]
  pub fn new(url: &str, timeout: Option<Duration>) -> SendSocket {
    let mut socket = Socket::new(Protocol::Push).unwrap();
    timeout.map(|timeout| socket.set_receive_timeout(as_millis(timeout)).unwrap());
    let endpoint = socket.connect(url).unwrap();

    SendSocket {
      socket: socket,
      endpoint: endpoint,
    }
  }

  /// Block until we can send this socket a message.
  pub fn write(&mut self, msg: &[u8]) -> std::io::Result<()> {
    self.socket.write(msg).map(|_| ())
  }

  /// Terminate this connection.
  pub fn close(self) {
    // The `drop` takes care of everything.
  }
}

impl Drop for SendSocket {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap_or(());
  }
}

#[allow(missing_docs)]
pub enum Result<T> {
  Success(T),
  Empty,
  Terminating,
}

/// A receive-only socket.
pub struct ReceiveSocket {
  socket: Socket,
  endpoint: Endpoint,
}

impl ReceiveSocket {
  #[allow(missing_docs)]
  pub fn new(url: &str, timeout: Option<Duration>) -> ReceiveSocket {
    let mut socket = Socket::new(Protocol::Pull).unwrap();
    timeout.map(|timeout| socket.set_receive_timeout(as_millis(timeout)).unwrap());
    let endpoint = socket.bind(url.as_ref()).unwrap();

    ReceiveSocket {
      socket: socket,
      endpoint: endpoint,
    }
  }

  /// Block until a message can be fetched from this socket.
  pub fn read(&mut self) -> Option<Vec<u8>> {
    let mut msg = Vec::new();
    if self.socket.read_to_end(&mut msg).is_ok() {
      Some(msg)
    } else {
      None
    }
  }

  /// Try to read a message from this socket.
  pub fn try_read(&mut self) -> Result<Vec<u8>> {
    let mut msg = Vec::new();
    let result = self.socket.nb_read_to_end(&mut msg);
    match result {
      Ok(_) => Result::Success(msg),
      Err(Error::TryAgain) => Result::Empty,
      Err(Error::Terminating) => Result::Terminating,
      Err(_) => {
        result.unwrap();
        unreachable!()
      }
    }
  }

  /// Terminate this connection.
  pub fn close(self) {
    // The `drop` takes care of everything.
  }
}

impl Drop for ReceiveSocket {
  fn drop(&mut self) {
    self.endpoint.shutdown().unwrap_or(());
  }
}
