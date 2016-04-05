//! Vertex data structures.

use cgmath::{Point2,Point3,Vector2};
#[cfg(test)]
use std::mem;

use common::color::Color4;

#[derive(Debug, Clone, Copy, PartialEq)]
/// An untextured rendering vertex, with position and color.
pub struct ColoredVertex {
  /// The 3-d position of this vertex in world space.
  pub position: Point3<f32>,
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
  pub fn square(min: Point2<f32>, max: Point2<f32>, color: Color4<f32>) -> [ColoredVertex; 6] {
    let vtx = |x, y| {
        ColoredVertex { position: Point3::new(x, y, 0.0), color: color }
      };

    [
      vtx(min.x, min.y), vtx(max.x, max.y), vtx(min.x, max.y),
      vtx(min.x, min.y), vtx(max.x, min.y), vtx(max.x, max.y),
    ]
  }
}

#[derive(Debug, Clone, Copy, PartialEq, RustcEncodable, RustcDecodable)]
/// A point in the world with corresponding texture data.
///
/// The texture position is [0, 1].
pub struct TextureVertex {
  /// The position of this vertex in the world.
  pub world_position:  Point3<f32>,

  /// The position of this vertex on a texture. The range of valid values
  /// in each dimension is [0, 1].
  pub texture_position: Vector2<f32>,
}
