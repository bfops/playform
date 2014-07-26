use gl::types::GLfloat;
use cgmath::vector;
use cgmath::vector::{Vector3};

#[deriving(Clone)]
pub struct BoundingBox {
  pub low_corner: vector::Vector3<GLfloat>,
  pub high_corner: vector::Vector3<GLfloat>,
}

pub type Intersect = Vector3<GLfloat>;

pub enum Intersect1 {
  Within,
  Partial,
}

// Find whether two Blocks intersect.
pub fn intersect(b1: &BoundingBox, b2: &BoundingBox) -> Option<Intersect> {
  fn intersect1(x1l: GLfloat, x1h: GLfloat, x2l: GLfloat, x2h: GLfloat) -> Option<Intersect1> {
    if x1l > x2l && x1h <= x2h {
      Some(Within)
    } else if x1h > x2l && x2h > x1l {
      Some(Partial)
    } else {
      None
    }
  }

  let mut ret = true;
  let mut v = Vector3::ident();
  match intersect1(b1.low_corner.x, b1.high_corner.x, b2.low_corner.x, b2.high_corner.x) {
    Some(Within) => { },
    Some(Partial) => { v.x = 0.0; },
    None => { ret = false; },
  }
  match intersect1(b1.low_corner.y, b1.high_corner.y, b2.low_corner.y, b2.high_corner.y) {
    Some(Within) => { },
    Some(Partial) => { v.y = 0.0; },
    None => { ret = false; },
  }
  match intersect1(b1.low_corner.z, b1.high_corner.z, b2.low_corner.z, b2.high_corner.z) {
    Some(Within) => { },
    Some(Partial) => { v.z = 0.0; },
    None => { ret = false; },
  }

  if ret {
    Some(v)
  } else {
    None
  }
}
