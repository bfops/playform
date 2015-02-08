use fontloader;
use nalgebra::Vec2;
use render_state::RenderState;
use vertex::TextureVertex;

pub fn make_text(render_state: &mut RenderState) {
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
