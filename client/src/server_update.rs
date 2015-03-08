use cgmath::{Aabb3, Point, Point3, Vector, Vector3};
use std::cmp::partial_max;
use std::f32::consts::PI;
use std::num::Float;

use common::block_position::BlockPosition;
use common::color::{Color3, Color4};
use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};
use common::surroundings_loader::LODChange;

use client::Client;
use light::Light;
use load_terrain::lod_index;
use vertex::ColoredVertex;
use view_update::ClientToView;

pub const TRIANGLES_PER_BOX: u32 = 12;
pub const VERTICES_PER_TRIANGLE: u32 = 3;
pub const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(0.5)
}

pub fn apply_server_update<UpdateView, UpdateServer, QueueBlock>(
  client: &Client,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
  queue_block: &mut QueueBlock,
  update: ServerToClient,
) where
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
  QueueBlock: FnMut(TerrainBlockSend),
{
  match update {
    ServerToClient::LeaseId(_) => {
      warn!("Client ID has already been leased.");
    },
    ServerToClient::PlayerAdded(id, _) => {
      warn!("Unexpected PlayerAdded event: {:?}.", id);
    },
    ServerToClient::UpdatePlayer(player_id, bounds) => {
      if player_id != client.player_id {
        return
      }

      let position = center(&bounds);

      *client.player_position.lock().unwrap() = position;
      update_view(ClientToView::MoveCamera(position));

      let position = BlockPosition::from_world_position(&position);
      client.surroundings_loader.lock().unwrap().update(
        position,
        |lod_change| {
          match lod_change {
            LODChange::Load(block_position, distance) => {
              let lod = lod_index(distance);
              update_server(ClientToServer::RequestBlock(block_position, lod));
            },
            LODChange::Unload(block_position) => {
              // The block removal code is duplicated elsewhere.
    
              client.loaded_blocks
                .lock().unwrap()
                .remove(&block_position)
                // If it wasn't loaded, don't unload anything.
                .map(|(block, prev_lod)| {
                  for id in block.ids.iter() {
                    update_view(ClientToView::RemoveTerrain(*id));
                  }
    
                  update_view(ClientToView::RemoveBlockData(block_position, prev_lod));
                });
            },
          };
        },
      );
    },
    ServerToClient::UpdateMob(id, bounds) => {
      let mesh = to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0));
      update_view(ClientToView::UpdateMob(id, mesh.iter().map(|&x| x).collect()));
    },
    ServerToClient::UpdateSun(fraction) => {
      // Convert to radians.
      let angle = fraction * 2.0 * PI;
      let (s, c) = angle.sin_cos();

      let sun_color =
        Color3::of_rgb(
          c.abs(),
          (s + 1.0) / 2.0,
          (s * 0.75 + 0.25).abs(),
        );

      let radius = 1024.0;
      let rel_position = Vector3::new(c, s, 0.0);
      rel_position.mul_s(radius);

      update_view(ClientToView::SetPointLight(
        Light {
          position: client.player_position.lock().unwrap().add_v(&rel_position),
          intensity: sun_color,
        }
      ));

      let ambient_light = partial_max(0.4, s / 2.0).unwrap();

      update_view(ClientToView::SetAmbientLight(
        Color3::of_rgb(
          sun_color.r * ambient_light,
          sun_color.g * ambient_light,
          sun_color.b * ambient_light,
        ),
      ));

      update_view(ClientToView::SetClearColor(sun_color));
    },
    ServerToClient::AddBlock(block) => {
      queue_block(block);
    },
  }
}

fn to_triangles(
  bounds: &Aabb3<f32>,
  c: &Color4<f32>,
) -> [ColoredVertex; TRIANGLE_VERTICES_PER_BOX as usize] {
  let (x1, y1, z1) = (bounds.min.x, bounds.min.y, bounds.min.z);
  let (x2, y2, z2) = (bounds.max.x, bounds.max.y, bounds.max.z);

  let vtx = |x, y, z| {
    ColoredVertex {
      position: Point3::new(x, y, z),
      color: c.clone(),
    }
  };

  // Remember: x increases to the right, y increases up, and z becomes more
  // negative as depth from the viewer increases.
  [
    // front
    vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
    vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
    // left
    vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
    vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
    // top
    vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
    vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
    // back
    vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
    vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
    // right
    vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
    vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
    // bottom
    vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
    vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
  ]
}
