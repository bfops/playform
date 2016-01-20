//! HUD initialization code.

use cgmath::Point2;

use common::color::Color4;

use vertex::{ColoredVertex};
use view;

/// Add HUD data into `view`.
pub fn make_hud<'a, 'b:'a>(view: &'a mut view::T<'b>) {
  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  let triangles: Vec<_> =
    ColoredVertex::square(
      Point2 { x: -0.02, y: -0.02 },
      Point2 { x:  0.02, y:  0.02 },
      cursor_color
    ).iter().cloned().collect();

  view.hud_triangles.bind(&mut view.gl);
  view.hud_triangles.push(&mut view.gl, triangles.as_ref());
}
