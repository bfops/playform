use rand;
use std;

use common::entity_id;
use common::id_allocator;
use common::protocol;
use common::voxel;

use chunk;
use edge;
use loaded_edges;
use terrain_mesh;
use view_update;

#[derive(Debug, Clone)]
pub enum Message {
  Chunk {
    chunk        : chunk::T,
    requested_at : u64,
  },
  Voxel {
    voxel : voxel::T,
  },
}

#[derive(Debug, Clone)]
enum InProgress {
  Edges(Message),
}

#[derive(Debug, Clone)]
pub struct T {
  queue       : std::collections::VecDeque<Message>,
  in_progress : Option<InProgress>,
}

impl T {
  pub fn enqueue(&mut self, msg: Message) {
    self.queue.push_back(msg);
  }

  pub fn tick<Rng, UpdateView>(
    &mut self,
    voxels       : &mut voxel::tree::T,
    rng          : &mut Rng,
    id_allocator : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
    loaded_edges : &mut loaded_edges::T<terrain_mesh::T>,
    update_view  : &mut UpdateView,
  ) where
    UpdateView : FnMut(view_update::T),
    Rng        : rand::Rng,
  {
    match self.in_progress {
      None => {
        let msg =
          match self.queue.pop_front() {
            None => return,
            Some(msg) => msg,
          };
        match msg {
          Message::Chunk { chunk, requested_at } => {
            for (bounds, voxel) in chunk.voxels() {
              voxels.get_mut_or_create(&bounds).data = Some(voxel);
            }
          },
          Message::Voxel { .. } => {
            unimplemented!();
          },
        }
        self.in_progress = Some(InProgress::Edges(msg));
      },
      Some(InProgress::Edges(ref msg)) => {
        match msg {
          &Message::Chunk { ref chunk, requested_at } => {
            for edge in chunk::edges(&chunk.position) {
              let _ =
                load_edge(
                  &chunk,
                  voxels,
                  rng,
                  id_allocator,
                  loaded_edges,
                  update_view,
                  &edge,
                );
            }
          },
          &Message::Voxel { .. } => {
            unimplemented!();
          },
        }
      },
    }
  }
}

fn load_edge<Rng, UpdateView>(
  chunk        : &chunk::T,
  voxels       : &mut voxel::tree::T,
  rng          : &mut Rng,
  id_allocator : &std::sync::Mutex<id_allocator::T<entity_id::T>>,
  loaded_edges : &mut loaded_edges::T<terrain_mesh::T>,
  update_view  : &mut UpdateView,
  edge         : &edge::T,
) -> Result<(), ()> where
  UpdateView : FnMut(view_update::T),
  Rng        : rand::Rng,
{
  let mut updates = Vec::new();

  let mesh_fragment = try!(terrain_mesh::generate(voxels, edge, id_allocator, rng));
  let unload_fragments = loaded_edges.insert(&edge, mesh_fragment.clone());

  for mesh_fragment in unload_fragments {
    for id in &mesh_fragment.ids {
      updates.push(view_update::RemoveTerrain(*id));
    }
    for id in &mesh_fragment.grass_ids {
      updates.push(view_update::RemoveGrass(*id));
    }
  }

  if !mesh_fragment.ids.is_empty() {
    updates.push(view_update::AddBlock(mesh_fragment));
  }

  update_view(view_update::Atomic(updates));

  Ok(())
}
