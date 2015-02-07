use fontloader;
use nalgebra::Vec2;
use renderer::Renderer;
use vertex::TextureVertex;

pub fn make_text(renderer: &mut Renderer) {
  let fontloader = fontloader::FontLoader::new();

  let instructions = vec!(
    "Use WASD to move, and spacebar to jump.",
    "Use the mouse to look around.",
  );

  let mut y = 0.99;

  renderer.text_triangles.bind(&mut renderer.gl);
  for line in instructions.iter() {
    renderer.text_textures.push(fontloader.sans.red(&mut renderer.gl, *line));

    renderer.text_triangles.push(
      &mut renderer.gl,
      &TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }
}
