use color::Color4;
use fontloader;
use nalgebra::Pnt2;
use nalgebra::Vec2;
use view::View;
use vertex::ColoredVertex;
use vertex::TextureVertex;

pub fn make_hud(view: &mut View) {
  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  view.hud_triangles.bind(&mut view.gl);
  view.hud_triangles.push(
    &mut view.gl,
    &ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    )
  );

  let fontloader = fontloader::FontLoader::new();

  let instructions = vec!(
    "Use WASD to move, and spacebar to jump.",
    "Use the mouse to look around.",
  );

  let mut y = 0.99;

  view.text_triangles.bind(&mut view.gl);
  for line in instructions.iter() {
    view.text_textures.push(fontloader.sans.red(&mut view.gl, *line));

    view.text_triangles.push(
      &mut view.gl,
      &TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }
}
