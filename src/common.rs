#![macro_escape]

use glw::color::Color4;
use glw::vertex::{ColoredVertex};
use nalgebra::na::Vec3;
use ncollide3df32::bounding_volume::aabb::AABB;

pub static WINDOW_WIDTH:  uint = 800;
pub static WINDOW_HEIGHT: uint = 600;

pub static TRIANGLES_PER_BOX: uint = 12;
pub static VERTICES_PER_TRIANGLE: uint = 3;
pub static TRIANGLE_VERTICES_PER_BOX: uint = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

pub static VERTICES_PER_LINE: uint = 2;
pub static LINES_PER_BOX: uint = 12;
pub static LINE_VERTICES_PER_BOX: uint = LINES_PER_BOX * VERTICES_PER_LINE;

pub fn partial_min_by<A: Copy, T: Iterator<A>, B: PartialOrd>(t: T, f: |A| -> B) -> Option<A> {
  let mut t = t;
  let (mut min_a, mut min_b) = {
    match t.next() {
      None => return None,
      Some(a) => (a, f(a)),
    }
  };
  for a in t {
    let b = f(a);
    assert!(b != min_b);
    if b < min_b {
      min_a = a;
      min_b = b;
    }
  }

  Some(min_a)
}

pub fn to_outlines<'a>(bounds: &AABB) -> [ColoredVertex, ..LINE_VERTICES_PER_BOX] {
  let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
  let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);
  let c = Color4::of_rgba(0.0, 0.0, 0.0, 1.0);

  let vtx = |x: f32, y: f32, z: f32| -> ColoredVertex {
    ColoredVertex {
      position: Vec3::new(x, y, z),
      color: c
    }
  };

  [
    vtx(x1, y1, z1), vtx(x2, y1, z1),
    vtx(x1, y2, z1), vtx(x2, y2, z1),
    vtx(x1, y1, z2), vtx(x2, y1, z2),
    vtx(x1, y2, z2), vtx(x2, y2, z2),

    vtx(x1, y1, z1), vtx(x1, y2, z1),
    vtx(x2, y1, z1), vtx(x2, y2, z1),
    vtx(x1, y1, z2), vtx(x1, y2, z2),
    vtx(x2, y1, z2), vtx(x2, y2, z2),

    vtx(x1, y1, z1), vtx(x1, y1, z2),
    vtx(x2, y1, z1), vtx(x2, y1, z2),
    vtx(x1, y2, z1), vtx(x1, y2, z2),
    vtx(x2, y2, z1), vtx(x2, y2, z2),
  ]
}

#[macro_export]
macro_rules! unwrap(
  ($option:expr) => (
    match $option {
      None => fail!("unwrap() on None"),
      Some(x) => x,
    }
  )
)
