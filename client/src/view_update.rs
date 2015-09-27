//! Define the updates passed from the client to the view.

use cgmath::Point3;
use stopwatch;

use common::block_position::BlockPosition;
use common::color::Color3;
use common::entity::EntityId;
use common::lod::LODIndex;
use common::terrain_block::TerrainBlock;

use light;
use light::{set_sun, set_ambient_light};
use mob_buffers::VERTICES_PER_MOB;
use player_buffers::VERTICES_PER_PLAYER;
use vertex::ColoredVertex;
use view::View;

/// Messages from the client to the view.
pub enum ClientToView {
  /// Set the camera location.
  MoveCamera(Point3<f32>),

  /// Update a player mesh.
  UpdatePlayer(EntityId, [ColoredVertex; VERTICES_PER_PLAYER]),
  /// Update a mob mesh.
  UpdateMob(EntityId, [ColoredVertex; VERTICES_PER_MOB]),

  /// Update the sun.
  SetSun(light::Sun),
  /// Update the ambient light.
  SetAmbientLight(Color3<f32>),
  /// Update the GL clear color.
  SetClearColor(Color3<f32>),

  /// Add a terrain block to the view.
  AddBlock(BlockPosition, TerrainBlock, LODIndex),
  /// Remove a terrain entity.
  RemoveTerrain(EntityId),
  /// Remove block-specific data.
  RemoveBlockData(BlockPosition, LODIndex),
  /// Treat a series of updates as an atomic operation.
  Atomic(Vec<ClientToView>),
}

unsafe impl Send for ClientToView {}

#[allow(missing_docs)]
pub fn apply_client_to_view(view: &mut View, up: ClientToView) {
  match up {
    ClientToView::MoveCamera(position) => {
      view.camera.translate_to(position);
    },
    ClientToView::UpdateMob(id, triangles) => {
      view.mob_buffers.insert(&mut view.gl, id, &triangles);
    },
    ClientToView::UpdatePlayer(id, triangles) => {
      view.player_buffers.insert(&mut view.gl, id, &triangles);
    },
    ClientToView::SetSun(sun) => {
      set_sun(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        &sun,
      );
    },
    ClientToView::SetAmbientLight(color) => {
      set_ambient_light(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        color,
      );
    },
    ClientToView::SetClearColor(color) => {
      view.gl.set_background_color(color.r, color.g, color.b, 1.0);
    },
    ClientToView::AddBlock(_, block, _) => {
      stopwatch::time("add_block", || {
        view.terrain_buffers.push(
          &mut view.gl,

          block.vertex_coordinates.as_ref(),
          block.normals.as_ref(),
          block.ids.as_ref(),
          block.materials.as_ref(),
        );
      })
    },
    ClientToView::RemoveTerrain(id) => {
      view.terrain_buffers.swap_remove(&mut view.gl, id);
    },
    ClientToView::RemoveBlockData(_, _) => {
    },
    ClientToView::Atomic(updates) => {
      for up in updates.into_iter() {
        apply_client_to_view(view, up);
      }
    },
  };
}
