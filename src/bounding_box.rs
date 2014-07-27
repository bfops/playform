use color::Color4;
use gl::types::GLfloat;
use cgmath::vector;
use cgmath::vector::{Vector3};
use cgmath::point::{Point3};
use vertex::{ColoredVertex};

pub static TRIANGLES_PER_BOX: uint = 12;
pub static LINES_PER_BOX: uint = 12;
pub static VERTICES_PER_TRIANGLE: uint = 3;
pub static VERTICES_PER_LINE: uint = 2;
pub static TRIANGLE_VERTICES_PER_BOX: uint = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;
pub static LINE_VERTICES_PER_BOX: uint = LINES_PER_BOX * VERTICES_PER_LINE;

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

impl BoundingBox {
  // Construct the faces of the box as triangles for rendering,
  // with a different color on each face (front, left, top, back, right, bottom).
  // Triangle vertices are in CCW order when viewed from the outside of
  // the cube, for rendering purposes.
  pub fn to_triangles(&self, c: [Color4<GLfloat>, ..6]) -> [ColoredVertex, ..VERTICES_PER_TRIANGLE * TRIANGLES_PER_BOX] {
    let (x1, y1, z1) = (self.low_corner.x, self.low_corner.y, self.low_corner.z);
    let (x2, y2, z2) = (self.high_corner.x, self.high_corner.y, self.high_corner.z);

    let vtx = |x: GLfloat, y: GLfloat, z: GLfloat, c: Color4<GLfloat>| -> ColoredVertex {
      ColoredVertex {
        position: Point3 { x: x, y: y, z: z },
        color: c
      }
    };

    [
      // front
      vtx(x1, y1, z1, c[0]), vtx(x1, y2, z1, c[0]), vtx(x2, y2, z1, c[0]),
      vtx(x1, y1, z1, c[0]), vtx(x2, y2, z1, c[0]), vtx(x2, y1, z1, c[0]),
      // left
      vtx(x1, y1, z2, c[1]), vtx(x1, y2, z2, c[1]), vtx(x1, y2, z1, c[1]),
      vtx(x1, y1, z2, c[1]), vtx(x1, y2, z1, c[1]), vtx(x1, y1, z1, c[1]),
      // top
      vtx(x1, y2, z1, c[2]), vtx(x1, y2, z2, c[2]), vtx(x2, y2, z2, c[2]),
      vtx(x1, y2, z1, c[2]), vtx(x2, y2, z2, c[2]), vtx(x2, y2, z1, c[2]),
      // back
      vtx(x2, y1, z2, c[3]), vtx(x2, y2, z2, c[3]), vtx(x1, y2, z2, c[3]),
      vtx(x2, y1, z2, c[3]), vtx(x1, y2, z2, c[3]), vtx(x1, y1, z2, c[3]),
      // right
      vtx(x2, y1, z1, c[4]), vtx(x2, y2, z1, c[4]), vtx(x2, y2, z2, c[4]),
      vtx(x2, y1, z1, c[4]), vtx(x2, y2, z2, c[4]), vtx(x2, y1, z2, c[4]),
      // bottom
      vtx(x1, y1, z2, c[5]), vtx(x1, y1, z1, c[5]), vtx(x2, y1, z1, c[5]),
      vtx(x1, y1, z2, c[5]), vtx(x2, y1, z1, c[5]), vtx(x2, y1, z2, c[5]),
    ]
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
}
