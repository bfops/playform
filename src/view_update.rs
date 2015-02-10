use color::Color3;
use nalgebra::{Pnt3, Vec3};
use light::{Light, set_point_light, set_ambient_light};
use std::cmp::partial_max;
use std::f32::consts::PI;
use std::iter::repeat;
use std::num::Float;
use terrain::terrain_block::{BlockPosition, TerrainBlock};
use vertex::ColoredVertex;
use view::View;
use world::EntityId;

#[derive(Clone)]
pub enum ViewUpdate {
  UpdatePlayer(Pnt3<f32>),

  AddMob((EntityId, Vec<ColoredVertex>)),
  UpdateMob((EntityId, Vec<ColoredVertex>)),

  // The sun as a [0, 1) portion of its cycle.
  UpdateSun(f32),

  AddBlock((BlockPosition, TerrainBlock, u32)),
  RemoveTerrain(EntityId),
  RemoveBlockData((BlockPosition, u32)),
}

impl ViewUpdate {
  pub fn apply(self, view: &mut View) {
    match self {
      ViewUpdate::UpdatePlayer(position) => {
        view.camera.translate_to(position);
      },
      ViewUpdate::AddMob((id, triangles)) => {
        view.mob_buffers.push(&mut view.gl, id, triangles.as_slice());
      },
      ViewUpdate::UpdateMob((id, triangles)) => {
        view.mob_buffers.update(&mut view.gl, id, triangles.as_slice());
      },
      ViewUpdate::UpdateSun(fraction) => {
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
        let rel_position = Vec3::new(c, s, 0.0) * radius;

        set_point_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          &Light {
            position: view.camera.position + rel_position,
            intensity: sun_color,
          }
        );

        let ambient_light = partial_max(0.4, s / 2.0).unwrap();

        set_ambient_light(
          &mut view.shaders.terrain_shader.shader,
          &mut view.gl,
          Color3::of_rgb(
            sun_color.r * ambient_light,
            sun_color.g * ambient_light,
            sun_color.b * ambient_light,
          ),
        );

        view.gl.set_background_color(sun_color.r, sun_color.g, sun_color.b, 1.0);
      },
      ViewUpdate::AddBlock((block_position, block, lod)) => {
        if !block.ids.is_empty() {
          let block_index =
            view.terrain_buffers.push_block_data(
              &mut view.gl,
              block_position,
              block.pixels.as_slice(),
              lod,
            );

          let block_indices: Vec<_> =
            repeat(block_index).take(block.ids.len()).collect();

          view.terrain_buffers.push(
            &mut view.gl,
            block.vertex_coordinates.as_slice(),
            block.normals.as_slice(),
            block.coords.as_slice(),
            block_indices.as_slice(),
            block.ids.as_slice(),
          );
        }
      },
      ViewUpdate::RemoveTerrain(id) => {
        view.terrain_buffers.swap_remove(&mut view.gl, id);
      },
      ViewUpdate::RemoveBlockData((block_position, lod)) => {
        view.terrain_buffers.free_block_data(lod, &block_position);
      },
    };
  }
}

unsafe impl Send for ViewUpdate {}
