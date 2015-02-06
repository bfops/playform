use common::*;
use fontloader;
use nalgebra::Vec2;
use vertex::TextureVertex;
use yaglw::vertex_buffer::*;
use yaglw::gl_context::{GLContext, GLContextExistence};
use yaglw::shader::Shader;
use yaglw::texture::Texture2D;

pub fn make_text<'a>(
  gl: &'a GLContextExistence,
  gl_context: &mut GLContext,
  shader: &Shader<'a>,
) -> (Vec<Texture2D<'a>>, GLArray<'a, TextureVertex>) {
  let fontloader = fontloader::FontLoader::new();
  let mut textures = Vec::new();
  let buffer = GLBuffer::new(gl, gl_context, 8 * VERTICES_PER_TRIANGLE as usize);
  let mut triangles =
    GLArray::new(
      gl,
      gl_context,
      shader,
      &[
        VertexAttribData { name: "position", size: 3, unit: GLType::Float },
        VertexAttribData { name: "texture_position", size: 2, unit: GLType::Float },
      ],
      DrawMode::Triangles,
      buffer,
    );

  let instructions =
    &[
      "Use WASD to move, and spacebar to jump.",
      "Use the mouse to look around.",
    ].to_vec();

  let mut y = 0.99;

  for line in instructions.iter() {
    textures.push(fontloader.sans.red(gl, *line));

    triangles.push(
      gl_context,
      &TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }

  (textures, triangles)
}

