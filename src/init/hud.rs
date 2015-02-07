use color::Color4;
use nalgebra::Pnt2;
use renderer::Renderer;
use vertex::ColoredVertex;

pub fn make_hud(renderer: &mut Renderer) {
  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  renderer.hud_triangles.bind(&mut renderer.gl);
  renderer.hud_triangles.push(
    &mut renderer.gl,
    &ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    )
  );
}
