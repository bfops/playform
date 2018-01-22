//! A low-level interface to send and receive server-client protocol messages

use std;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::sync::atomic::{Ordering, AtomicUsize};

use bincode;

use common::protocol;
use common::socket::{SendSocket, ReceiveSocket};

#[allow(missing_docs)]
#[derive(Clone)]
pub struct SSender {
  // A boxed slice is used to reduce the sent size
  pub sender: Sender<Box<[u8]>>,
  // Please replace with AtomicU64 when it becomes stable
  pub bytes_sent: Arc<AtomicUsize>,
}

impl SSender {
  #[allow(missing_docs)]
  pub fn new(sender: Sender<Box<[u8]>>) -> SSender {
    SSender {
      sender: sender,
      bytes_sent: Arc::new(AtomicUsize::new(0)),
    }
  }

  #[allow(missing_docs)]
  pub fn tell(&self, msg: &protocol::ClientToServer) {
    let msg = bincode::serialize(msg, bincode::Infinite).unwrap();
    // We aren't reading this until long after the write, so we use `Relaxed`
    self.bytes_sent.fetch_add(msg.len() as usize, Ordering::Relaxed);
    self.sender.send(msg.into_boxed_slice()).unwrap();
  }
}

#[allow(missing_docs)]
#[derive(Clone)]
pub struct SReceiver (Arc<Mutex<Receiver<Box<[u8]>>>>);

impl SReceiver {
  #[allow(missing_docs)]
  pub fn try(&self) -> Option<protocol::ServerToClient> {
    match self.0.lock().unwrap().try_recv() {
      Ok(msg) => Some(bincode::deserialize(&Vec::from(msg)).unwrap()),
      Err(TryRecvError::Empty) => None,
      e => {
        e.unwrap();
        unreachable!();
      },
    }
  }

  #[allow(missing_docs)]
  pub fn wait(&self) -> protocol::ServerToClient {
    let msg = self.0.lock().unwrap().recv().unwrap();
    bincode::deserialize(msg.as_ref()).unwrap()
  }
}

#[allow(missing_docs)]
#[derive(Clone)]
pub struct T {
  pub talk   : SSender,
  pub listen : SReceiver,
}

#[allow(missing_docs)]
pub fn new(
  server_url: &str,
  listen_url: &str,
) -> T {
  let (send_send, send_recv) = std::sync::mpsc::channel();
  let (recv_send, recv_recv) = std::sync::mpsc::channel();

  let _recv_thread ={
    let listen_url = listen_url.to_owned();
    let recv_send = recv_send.clone();
    std::thread::spawn(move || {
      let mut listen_socket =
        ReceiveSocket::new(
          listen_url.clone().as_ref(),
          Some(std::time::Duration::from_secs(30)),
        );
      loop {
        match listen_socket.read() {
          None => break,
          Some(msg) => {
            recv_send.send(msg.into_boxed_slice()).unwrap()
          },
        }
      }
    })
  };

  let _send_thread = {
    let server_url = server_url.to_owned();
    std::thread::spawn(move || {
      let mut talk_socket =
        SendSocket::new(
          server_url.as_ref(),
          Some(std::time::Duration::from_secs(30)),
        );
      loop {
        match send_recv.recv() {
          Err(_) => break,
          Ok(msg) => {
            let msg = Vec::from(msg);
            talk_socket.write(msg.as_ref()).unwrap();
          },
        }
      }
    })
  };

  T {
    talk: SSender::new(send_send),
    listen: SReceiver(Arc::new(Mutex::new(recv_recv))),
  }
}
