//! A vertex with and without textures attached.
use gl::types::GLfloat;
use cgmath::point::{Point2,Point3};
use cgmath::aabb::Aabb2;
use color::Color4;

#[deriving(Clone, Copy)]
/// An untextured rendering vertex, with position and color.
pub struct ColoredVertex {
  /// The 3-d position of this vertex in world space.
  pub position: Point3<GLfloat>,
  /// The color to apply to this vertex, in lieu of a texture.
  pub color:    Color4<GLfloat>,
}

impl ColoredVertex {
  /// Generates two colored triangles, representing a square, at z=0.
  /// The bounds of the square is represented by `b`.
  pub fn square(b: Aabb2<GLfloat>, color: Color4<GLfloat>) -> [ColoredVertex, ..6] {
    let vtx = |x, y| {
        ColoredVertex { position: Point3 { x: x, y: y, z: 0.0 }, color: color }
      };

    [
      vtx(b.min.x, b.min.y), vtx(b.max.x, b.max.y), vtx(b.min.x, b.max.y),
      vtx(b.min.x, b.min.y), vtx(b.max.x, b.min.y), vtx(b.max.x, b.max.y),
    ]
  }
}

#[deriving(Clone, Copy)]
/// A point on a texture, with both a screen position and a texture position.
///
/// The screen position is from [-1, 1], and the texture position is [0, 1].
/// This is opengl's fault, not mine. Don't shoot the messenger.
pub struct TextureVertex {
  /// The position of this vertex on the screen. The range of valid values
  /// in each dimension is [-1, 1].
  pub screen_position:  Point2<GLfloat>,

  /// The position of this vertex on a texture. The range of valid values
  /// in each dimension is [0, 1].
  pub texture_position: Point2<GLfloat>,
}

impl TextureVertex {
  /// Generates two textured triangles, representing a square in 2D space.
  /// The bounds of the square is represented by `b`.
  ///
  /// The coordinates on the texture will implicitly be the "whole thing".
  /// i.e. [(0, 0), (1, 1)].
  pub fn square(b: Aabb2<GLfloat>) -> [TextureVertex, ..6] {
    let vtx = |x, y, tx, ty| {
        TextureVertex {
          screen_position:  Point2 { x: x,  y: y, },
          texture_position: Point2 { x: tx, y: ty },
        }
      };

    [
      vtx(b.min.x, b.min.y, 0.0, 0.0),
      vtx(b.max.x, b.max.y, 1.0, 1.0),
      vtx(b.min.x, b.max.y, 0.0, 1.0),

      vtx(b.min.x, b.min.y, 0.0, 0.0),
      vtx(b.max.x, b.min.y, 1.0, 0.0),
      vtx(b.max.x, b.max.y, 1.0, 1.0),
    ]
  }
}
