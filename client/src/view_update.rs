//! Define the updates passed from the client to the view.

use common::color::Color3;
use common::communicate::TerrainBlockSend;
use common::entity::EntityId;
use common::lod::LODIndex;
use common::block_position::BlockPosition;
use common::vertex::ColoredVertex;
use light::{Light, set_point_light, set_ambient_light};
use cgmath::Point3;
use std::iter::repeat;
use view::View;

#[derive(Clone)]
/// Messages from the client to the view.
pub enum ClientToView {
  /// Set the camera location.
  MoveCamera(Point3<f32>),

  /// Add a mob mesh to the view.
  AddMob(EntityId, Vec<ColoredVertex>),
  /// Update a mob mesh in the view.
  UpdateMob(EntityId, Vec<ColoredVertex>),

  /// Update the point light.
  SetPointLight(Light),
  /// Update the ambient light.
  SetAmbientLight(Color3<f32>),
  /// Update the GL clear color.
  SetClearColor(Color3<f32>),

  /// Add a terrain block to the view.
  AddBlock(TerrainBlockSend),
  /// Remove a terrain entity.
  RemoveTerrain(EntityId),
  /// Remove block-specific data.
  RemoveBlockData(BlockPosition, LODIndex),
}

unsafe impl Send for ClientToView {}

#[allow(missing_docs)]
pub fn apply_client_to_view(up: ClientToView, view: &mut View) {
  match up {
    ClientToView::MoveCamera(position) => {
      view.camera.translate_to(position);
    },
    ClientToView::AddMob(id, triangles) => {
      view.mob_buffers.push(&mut view.gl, id, triangles.as_slice());
    },
    ClientToView::UpdateMob(id, triangles) => {
      view.mob_buffers.update(&mut view.gl, id, triangles.as_slice());
    },
    ClientToView::SetPointLight(light) => {
      set_point_light(
        &mut view.shaders.terrain_shader.shader,
        &mut view.gl,
        &light
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
    ClientToView::AddBlock(block) => {
      let block_index =
        view.terrain_buffers.push_block_data(
          &mut view.gl,
          block.position,
          block.block.pixels.as_slice(),
          block.lod,
        );

      let block_indices: Vec<_> =
        repeat(block_index).take(block.block.ids.len()).collect();

      view.terrain_buffers.push(
        &mut view.gl,
        block.block.vertex_coordinates.as_slice(),
        block.block.normals.as_slice(),
        block.block.coords.as_slice(),
        block_indices.as_slice(),
        block.block.ids.as_slice(),
      );
    },
    ClientToView::RemoveTerrain(id) => {
      view.terrain_buffers.swap_remove(&mut view.gl, id);
    },
    ClientToView::RemoveBlockData(block_position, lod) => {
      view.terrain_buffers.free_block_data(lod, &block_position);
    },
  };
}
