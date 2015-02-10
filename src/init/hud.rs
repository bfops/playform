use color::Color4;
use nalgebra::Pnt2;
use nalgebra::Vec2;
use std::sync::mpsc::Sender;
use view_update::ViewUpdate;
use view_update::ViewUpdate::*;
use vertex::ColoredVertex;
use vertex::TextureVertex;

pub fn make_hud(view: &Sender<ViewUpdate>) {
  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  let triangles =
    ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    ).iter().map(|&x| x).collect();

  view.send(PushHudTriangles(triangles)).unwrap();

  let instructions = vec!(
    "Use WASD to move, and spacebar to jump.",
    "Use the mouse to look around.",
  );

  let mut y = 0.99;

  for line in instructions.iter() {
    view.send(PushText(
      (Color4::of_rgba(0xFF, 0, 0, 0xFF), String::from_str(line))
    )).unwrap();

    let triangles =
      TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
      .iter()
      .map(|&x| x)
      .collect();
    view.send(PushTextTriangles(triangles)).unwrap();
    y -= 0.2;
  }
}
