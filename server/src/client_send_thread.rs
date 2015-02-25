use common::communicate::ServerToClient;
use common::socket::SendSocket;

pub fn client_send_thread<Recv>(
  client_url: String,
  recv: &mut Recv,
) where
  Recv: FnMut() -> Option<ServerToClient>,
{
  let mut socket = SendSocket::new(client_url.as_slice());
  while let Some(msg) = recv() {
    socket.write(msg);
  }
}
