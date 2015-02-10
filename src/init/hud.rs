use color::Color4;
use nalgebra::Pnt2;
use nalgebra::Vec2;
use view::View;
use vertex::ColoredVertex;
use vertex::TextureVertex;

pub fn make_hud(view: &mut View) {
  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  let triangles: Vec<_> =
    ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    ).iter().map(|&x| x).collect();

  view.hud_triangles.bind(&mut view.gl);
  view.hud_triangles.push(&mut view.gl, triangles.as_slice());

  let instructions = vec!(
    "Use WASD to move, and spacebar to jump.",
    "Use the mouse to look around.",
  );

  let mut y = 0.99;

  for line in instructions.iter() {
    let tex = view.fontloader.sans.render(
      &view.gl,
      line,
      Color4::of_rgba(0xFF, 0, 0, 0xFF),
    );
    view.text_textures.push(tex);

    let triangles: Vec<_> =
      TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
      .iter()
      .map(|&x| x)
      .collect();
    view.text_triangles.bind(&mut view.gl);
    view.text_triangles.push(&mut view.gl, triangles.as_slice());
    y -= 0.2;
  }
}
