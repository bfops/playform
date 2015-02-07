use color::Color4;
use common::*;
use nalgebra::Pnt2;
use vertex::ColoredVertex;
use yaglw::vertex_buffer::*;
use yaglw::gl_context::GLContext;
use yaglw::shader::Shader;

pub fn make_hud<'a, 'b:'a>(
  gl: &'a mut GLContext,
  shader: &Shader<'b>,
) -> GLArray<'b, ColoredVertex> {
  let buffer = GLBuffer::new(gl, 16 * VERTICES_PER_TRIANGLE as usize);
  let mut hud_triangles = {
    GLArray::new(
      gl,
      shader,
      &[
        VertexAttribData { name: "position", size: 3, unit: GLType::Float },
        VertexAttribData { name: "in_color", size: 4, unit: GLType::Float },
      ],
      DrawMode::Triangles,
      buffer,
    )
  };

  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  hud_triangles.push(
    gl,
    &ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    )
  );

  hud_triangles
}

