use color::Color4;
use fontloader;
use nalgebra::Pnt2;
use nalgebra::Vec2;
use render_state::RenderState;
use vertex::ColoredVertex;
use vertex::TextureVertex;

pub fn make_hud(render_state: &mut RenderState) {
  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  render_state.hud_triangles.bind(&mut render_state.gl);
  render_state.hud_triangles.push(
    &mut render_state.gl,
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

  render_state.text_triangles.bind(&mut render_state.gl);
  for line in instructions.iter() {
    render_state.text_textures.push(fontloader.sans.red(&mut render_state.gl, *line));

    render_state.text_triangles.push(
      &mut render_state.gl,
      &TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }
}
