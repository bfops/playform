//! RGBA color.

#[derive(Show, Clone, Copy, PartialEq)]
/// A color with red, green, blue, and alpha channels.
pub struct Color4<T> {
  /// Red
  pub r: T,
  /// Green
  pub g: T,
  /// Blue
  pub b: T,
  /// Alpha
  pub a: T,
}

impl<T: Copy> Color4<T> {
  /// Constructs a new `Color4` out of its RGBA values.
  pub fn of_rgba(r: T, g: T, b: T, a: T) -> Color4<T> {
    Color4 { r: r, g: g, b: b, a: a }
  }
}

