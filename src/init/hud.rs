use color::Color4;
use nalgebra::Pnt2;
use render_state::RenderState;
use vertex::ColoredVertex;

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
}
