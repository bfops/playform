use gl::types::*;
use color::Color4;
use nalgebra::Pnt3;
use ncollide::bounding_volume::AABB3;
use vertex::ColoredVertex;

pub const WINDOW_WIDTH:  u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

pub const TRIANGLES_PER_BOX: u32 = 12;
pub const VERTICES_PER_TRIANGLE: u32 = 3;
pub const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

pub fn partial_min_by<A, I, B, F>(mut iter: I, f: F) -> Vec<A>
    where A: Copy, I: Iterator<Item=A>, B: PartialOrd, F: Fn(A) -> B {
  let mut min_a = Vec::new();
  let mut min_b = {
    match iter.next() {
      None => return min_a,
      Some(a) => {
        min_a.push(a);
        f(a)
      }
    }
  };
  for a in iter {
    let b = f(a);
    if b < min_b {
      min_a.truncate(0);
      min_a.push(a);
      min_b = b;
    } else if b == min_b {
      min_a.push(a);
    }
  }

  min_a
}

pub fn to_triangles(
  bounds: &AABB3<GLfloat>,
  c: &Color4<GLfloat>,
) -> [ColoredVertex; (VERTICES_PER_TRIANGLE * TRIANGLES_PER_BOX) as usize] {
  let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
  let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

  let vtx = |&:x, y, z| {
    ColoredVertex {
      position: Pnt3::new(x, y, z),
      color: c.clone(),
    }
  };

  // Remember: x increases to the right, y increases up, and z becomes more
  // negative as depth from the viewer increases.
  [
    // front
    vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
    vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
    // left
    vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
    vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
    // top
    vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
    vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
    // back
    vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
    vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
    // right
    vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
    vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
    // bottom
    vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
    vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
  ]
}
