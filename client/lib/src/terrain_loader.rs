use common::chunk;
use common::protocol;

pub struct Message {
  chunk_position : chunk::Position,
  chunk          : chunk::T,
  reason         : protocol::VoxelReason,
  request_time   : Option<u64>,
}

enum InProgress {
  Edges(chunk::Edges),
}

pub struct T {
  queue          : std::collections::VecDeque<Message>,
  in_progress    : Option<(chunk::Position, InProgress)>,
}

impl T {
  pub fn enqueue(&mut self, msg: Message) {
    self.queue.push_back(msg);
  }

  pub fn tick(&mut self, voxels: &mut voxel::tree::T) {
    match self.in_progress {
      None => {
        let msg = self.queue.pop_front();
      }
      Some(ref position, Edges(ref edges)) => {
      },
    }
  }
}
