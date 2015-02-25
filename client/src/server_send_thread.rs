use common::communicate::ClientToServer;

pub fn server_send_thread<Recv, Talk>(
  recv: &mut Recv,
  talk: &mut Talk,
) where
  Recv: FnMut() -> Option<ClientToServer>,
  Talk: FnMut(ClientToServer),
{
  while let Some(msg) = recv() {
    talk(msg);
  }
}
