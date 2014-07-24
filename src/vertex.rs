//! A vertex with and without textures attached.
use gl::types::GLfloat;
use cgmath::point::{Point2,Point3};
use color::Color4;

#[deriving(Clone)]
/// An untextured rendering vertex, with position and color.
pub struct ColoredVertex {
  /// The 3-d position of this vertex in world space.
  pub position: Point3<GLfloat>,
  /// The color to apply to this vertex, in lieu of a texture.
  pub color:    Color4<GLfloat>,
}

#[deriving(Clone)]
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
