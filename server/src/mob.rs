use common::color::Color4;
use common::entity::EntityId;
use common::lod::OwnerId;
use common::surroundings_loader::SurroundingsLoader;
use common::vertex::ColoredVertex;
use nalgebra::{Pnt3, Vec3};
use ncollide_entities::bounding_volume::AABB3;
use server::Server;

pub const TRIANGLES_PER_BOX: u32 = 12;
pub const VERTICES_PER_TRIANGLE: u32 = 3;
pub const TRIANGLE_VERTICES_PER_BOX: u32 = TRIANGLES_PER_BOX * VERTICES_PER_TRIANGLE;

pub type Behavior = fn(&Server, &mut Mob);

pub struct Mob<'a> {
  pub position: Pnt3<f32>,
  pub speed: Vec3<f32>,
  pub behavior: Behavior,

  pub entity_id: EntityId,
  pub owner_id: OwnerId,

  // Nearby blocks should be made solid if they aren't loaded yet.
  pub solid_boundary: SurroundingsLoader<'a>,
}

impl<'a> Mob<'a> {
  pub fn to_triangles(
    bounds: &AABB3<f32>,
    c: &Color4<f32>,
  ) -> [ColoredVertex; TRIANGLE_VERTICES_PER_BOX as usize] {
    let (x1, y1, z1) = (bounds.mins().x, bounds.mins().y, bounds.mins().z);
    let (x2, y2, z2) = (bounds.maxs().x, bounds.maxs().y, bounds.maxs().z);

    let vtx = |&:x, y, z| {
      ColoredVertex {
        position: Pnt3::new(x, y, z),
        color: c.clone(),
      }
    };

    // Remember: x increases to the right, y increases up, and z becomes more
    // negative as depth from the viewer increases.
    [
      // front
      vtx(x1, y1, z2), vtx(x2, y2, z2), vtx(x1, y2, z2),
      vtx(x1, y1, z2), vtx(x2, y1, z2), vtx(x2, y2, z2),
      // left
      vtx(x1, y1, z1), vtx(x1, y2, z2), vtx(x1, y2, z1),
      vtx(x1, y1, z1), vtx(x1, y1, z2), vtx(x1, y2, z2),
      // top
      vtx(x1, y2, z1), vtx(x2, y2, z2), vtx(x2, y2, z1),
      vtx(x1, y2, z1), vtx(x1, y2, z2), vtx(x2, y2, z2),
      // back
      vtx(x1, y1, z1), vtx(x2, y2, z1), vtx(x2, y1, z1),
      vtx(x1, y1, z1), vtx(x1, y2, z1), vtx(x2, y2, z1),
      // right
      vtx(x2, y1, z1), vtx(x2, y2, z2), vtx(x2, y1, z2),
      vtx(x2, y1, z1), vtx(x2, y2, z1), vtx(x2, y2, z2),
      // bottom
      vtx(x1, y1, z1), vtx(x2, y1, z2), vtx(x1, y1, z2),
      vtx(x1, y1, z1), vtx(x2, y1, z1), vtx(x2, y1, z2),
    ]
  }
}
