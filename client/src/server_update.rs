use cgmath::{Aabb3, Point, Point3, Vector, Vector3};
use std::f32;
use std::f32::consts::PI;
use stopwatch;

use common::color::{Color3, Color4};
use common::communicate::{ClientToServer, ServerToClient, TerrainBlockSend};
use common::serialize::Copyable;

use client;
use light;
use vertex::ColoredVertex;
use view_update::ClientToView;

pub const TRIANGLES_PER_BOX: u32 = 12;
pub const VERTICES_PER_TRIANGLE: u32 = 3;
pub const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

fn center(bounds: &Aabb3<f32>) -> Point3<f32> {
  bounds.min.add_v(&bounds.max.to_vec()).mul_s(0.5)
}

pub fn apply_server_update<UpdateView, UpdateServer, QueueBlock>(
  client: &client::T,
  update_view: &mut UpdateView,
  update_server: &mut UpdateServer,
  queue_block: &mut QueueBlock,
  update: ServerToClient,
) where
  UpdateView: FnMut(ClientToView),
  UpdateServer: FnMut(ClientToServer),
  QueueBlock: FnMut(TerrainBlockSend),
{
  stopwatch::time("apply_server_update", move || {
    match update {
      ServerToClient::LeaseId(_) => {
        warn!("Client ID has already been leased.");
      },
      ServerToClient::Ping(Copyable(())) => {
        update_server(ClientToServer::Ping(Copyable(client.id)));
      },
      ServerToClient::PlayerAdded(Copyable(id), _) => {
        warn!("Unexpected PlayerAdded event: {:?}.", id);
      },
      ServerToClient::UpdatePlayer(Copyable(player_id), Copyable(bounds)) => {
        let mesh = to_triangles(&bounds, &Color4::of_rgba(0.0, 0.0, 1.0, 1.0));
        update_view(ClientToView::UpdatePlayer(player_id, mesh));

        // We "lock" the client to client.player_id, so for updates to that player only,
        // there is more client-specific logic.
        if player_id != client.player_id {
          return
        }

        let position = center(&bounds);

        *client.player_position.lock().unwrap() = position;
        update_view(ClientToView::MoveCamera(position));
      },
      ServerToClient::UpdateMob(Copyable(id), Copyable(bounds)) => {
        let mesh = to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0));
        update_view(ClientToView::UpdateMob(id, mesh));
      },
      ServerToClient::UpdateSun(Copyable(fraction)) => {
        // Convert to radians.
        let angle = fraction * 2.0 * PI;
        let (s, c) = angle.sin_cos();

        let sun_color =
          Color3::of_rgb(
            c.abs(),
            (s + 1.0) / 2.0,
            (s * 0.75 + 0.25).abs(),
          );

        update_view(ClientToView::SetSun(
          light::Sun {
            direction: Vector3::new(c, s, 0.0),
            intensity: sun_color,
          }
        ));

        let ambient_light = f32::max(0.4, s / 2.0);

        update_view(ClientToView::SetAmbientLight(
          Color3::of_rgb(
            sun_color.r * ambient_light,
            sun_color.g * ambient_light,
            sun_color.b * ambient_light,
          ),
        ));

        update_view(ClientToView::SetClearColor(sun_color));
      },
      ServerToClient::UpdateBlock(block) => {
        queue_block(block);
      },
    }
  })
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
