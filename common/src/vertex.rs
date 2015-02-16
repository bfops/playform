//! Vertex data structures.

use color::Color4;
use nalgebra::{Pnt2,Pnt3,Vec2,Vec3};
#[cfg(test)]
use std::mem;

#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(RustcDecodable, RustcEncodable)]
/// An untextured rendering vertex, with position and color.
pub struct ColoredVertex {
  /// The 3-d position of this vertex in world space.
  pub position: Pnt3<f32>,
  /// The color to apply to this vertex, in lieu of a texture.
  pub color:    Color4<f32>,
}

#[test]
fn check_vertex_size() {
  assert_eq!(mem::size_of::<ColoredVertex>(), 7*4);
  assert_eq!(mem::size_of::<TextureVertex>(), 5*4);
}

impl ColoredVertex {
  /// Generates two colored triangles, representing a square, at z=0.
  /// The bounds of the square is represented by `b`.
  pub fn square(min: Pnt2<f32>, max: Pnt2<f32>, color: Color4<f32>) -> [ColoredVertex; 6] {
    let vtx = |&: x, y| {
        ColoredVertex { position: Pnt3::new(x, y, 0.0), color: color }
      };

    [
      vtx(min.x, min.y), vtx(max.x, max.y), vtx(min.x, max.y),
      vtx(min.x, min.y), vtx(max.x, min.y), vtx(max.x, max.y),
    ]
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A point in the world with corresponding texture data.
///
/// The texture position is [0, 1].
pub struct TextureVertex {
  /// The position of this vertex in the world.
  pub world_position:  Vec3<f32>,

  /// The position of this vertex on a texture. The range of valid values
  /// in each dimension is [0, 1].
  pub texture_position: Vec2<f32>,
}

impl TextureVertex {
  /// Generates two textured triangles, representing a square in 2D space,
  /// at z = 0.
  /// The bounds of the square is represented by `b`.
  ///
  /// The coordinates on the texture will implicitly be the "whole thing".
  /// i.e. [(0, 0), (1, 1)].
  pub fn square(min: Vec2<f32>, max: Vec2<f32>) -> [TextureVertex; 6] {
    let vtx = |&: x, y, tx, ty| {
        TextureVertex {
          world_position:  Vec3::new(x, y, 0.0),
          texture_position: Vec2::new(tx, ty),
        }
      };

    [
      vtx(min.x, min.y, 0.0, 0.0),
      vtx(max.x, max.y, 1.0, 1.0),
      vtx(min.x, max.y, 0.0, 1.0),

      vtx(min.x, min.y, 0.0, 0.0),
      vtx(max.x, min.y, 1.0, 0.0),
      vtx(max.x, max.y, 1.0, 1.0),
    ]
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(RustcDecodable, RustcEncodable)]
/// A point in the world with corresponding texture and normal data.
///
/// The texture position is [0, 1].
pub struct ServerTextureVertex {
  /// The position of this vertex in the world.
  pub world_position:  Vec3<f32>,

  /// The position of this vertex on a texture. The range of valid values
  /// in each dimension is [0, 1].
  pub texture_position: Vec2<f32>,

  /// The normal vector for this vertex. We assume the length is 1.
  pub normal: Vec3<f32>,
}
