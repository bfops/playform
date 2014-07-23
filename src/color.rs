#[deriving(Clone, Copy)]
pub struct Color4<T> { pub r: T, pub g: T, pub b: T, pub a: T }

impl<T: Copy> Color4<T> {
  pub fn new(r: T, g: T, b: T, a: T) -> Color4<T> {
    Color4 { r: r, g: g, b: b, a: a }
  }
}

