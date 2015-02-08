use color::Color3;
use gl::types::*;
use id_allocator::IdAllocator;
use nalgebra::{Pnt2, Pnt3, Vec2, Vec3};
use ncollide_entities::bounding_volume::{AABB, AABB3};
use opencl_context::CL;
use world::EntityId;
use std::cmp::partial_max;
use std::num::Float;
use std::ops::Add;
use stopwatch::TimerSet;
use terrain::terrain::LOD_QUALITY;
use terrain::heightmap::HeightMap;
use terrain::texture_generator;
use terrain::texture_generator::TerrainTextureGenerator;
use terrain::tree_placer::TreePlacer;

pub const BLOCK_WIDTH: i32 = 8;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct BlockPosition(Pnt3<i32>);

impl BlockPosition {
  #[inline(always)]
  pub fn new(x: i32, y: i32, z: i32) -> BlockPosition {
    BlockPosition(Pnt3::new(x, y, z))
  }

  #[inline(always)]
  pub fn as_pnt<'a>(&'a self) -> &'a Pnt3<i32> {
    let BlockPosition(ref pnt) = *self;
    pnt
  }

  #[inline(always)]
  pub fn as_mut_pnt3<'a>(&'a mut self) -> &'a mut Pnt3<i32> {
    let BlockPosition(ref mut pnt) = *self;
    pnt
  }

  pub fn from_world_position(world_position: &Pnt3<f32>) -> BlockPosition {
    macro_rules! convert_coordinate(
      ($x: expr) => ({
        let x = $x.floor() as i32;
        let x =
          if x < 0 {
            x - (BLOCK_WIDTH - 1)
          } else {
            x
          };
        x / BLOCK_WIDTH
      })
    );
    BlockPosition(
      Pnt3::new(
        convert_coordinate!(world_position.x),
        convert_coordinate!(world_position.y),
        convert_coordinate!(world_position.z),
      )
    )
  }

  pub fn to_world_position(&self) -> Pnt3<f32> {
    Pnt3::new(
      (self.as_pnt().x * BLOCK_WIDTH) as f32,
      (self.as_pnt().y * BLOCK_WIDTH) as f32,
      (self.as_pnt().z * BLOCK_WIDTH) as f32,
    )
  }
}

impl Add<Vec3<i32>> for BlockPosition {
  type Output = BlockPosition;

  fn add(mut self, rhs: Vec3<i32>) -> Self {
    self.as_mut_pnt3().x += rhs.x;
    self.as_mut_pnt3().y += rhs.y;
    self.as_mut_pnt3().z += rhs.z;
    self
  }
}

pub struct TerrainBlock {
  // These Vecs must all be ordered the same way; each entry is the next triangle.

  pub vertex_coordinates: Vec<[Pnt3<GLfloat>; 3]>,
  pub normals: Vec<[Vec3<GLfloat>; 3]>,
  // per-vertex 2D coordinates into `pixels`.
  pub coords: Vec<[Pnt2<f32>; 3]>,

  pub pixels: Vec<Color3<f32>>,

  // per-triangle entity IDs
  pub ids: Vec<EntityId>,
  // per-triangle bounding boxes
  // TODO: Change this back to a HashMap once initial capacity is zero for those.
  pub bounds: Vec<(EntityId, AABB3<GLfloat>)>,
}

impl TerrainBlock {
  pub fn empty() -> TerrainBlock {
    TerrainBlock {
      vertex_coordinates: Vec::new(),
      normals: Vec::new(),
      coords: Vec::new(),

      pixels: Vec::new(),

      ids: Vec::new(),
      bounds: Vec::new(),
    }
  }

  pub fn generate(
    timers: &TimerSet,
    cl: &CL,
    id_allocator: &mut IdAllocator<EntityId>,
    heightmap: &HeightMap,
    texture_generator: &TerrainTextureGenerator,
    treemap: &TreePlacer,
    position: &BlockPosition,
    lod_index: u32,
  ) -> TerrainBlock {
    timers.time("update.generate_block", || {
      let mut block = TerrainBlock::empty();

      let position = position.to_world_position();

      let lateral_samples = LOD_QUALITY[lod_index as usize] as u8;
      let sample_width = BLOCK_WIDTH as f32 / lateral_samples as f32;

      let mut any_tiles = false;
      for dx in range(0, lateral_samples) {
        let dx = dx as f32;
        for dz in range(0, lateral_samples) {
          let dz = dz as f32;
          let tex_sample =
            texture_generator::TEXTURE_WIDTH[lod_index as usize] as f32 / lateral_samples as f32;
          let tex_coord = Pnt2::new(dx, dz) * tex_sample;
          let tile_position = position + Vec3::new(dx, 0.0, dz) * sample_width;
          if TerrainBlock::add_tile(
            timers,
            heightmap,
            treemap,
            id_allocator,
            &mut block,
            sample_width,
            tex_sample,
            &tile_position,
            tex_coord,
            lod_index,
          ) {
            any_tiles = true;
          }
        }
      }

      if any_tiles {
        block.pixels =
          texture_generator.generate(
            cl,
            position.x as f32,
            position.z as f32,
          );
      }

      block
    })
  }

  fn add_tile<'a>(
    timers: &TimerSet,
    hm: &HeightMap,
    treemap: &TreePlacer,
    id_allocator: &mut IdAllocator<EntityId>,
    block: &mut TerrainBlock,
    sample_width: f32,
    tex_sample: f32,
    position: &Pnt3<f32>,
    tex_coord: Pnt2<f32>,
    lod_index: u32,
  ) -> bool {
    let half_width = sample_width / 2.0;
    let center = hm.point_at(position.x + half_width, position.z + half_width);

    if position.y >= center.y || center.y > position.y + BLOCK_WIDTH as f32 {
      return false;
    }

    timers.time("update.generate_block.add_tile", || {
      let normal_delta = sample_width / 2.0;
      let center_normal =
        hm.normal_at(normal_delta, position.x + half_width, position.z + half_width);

      let x2 = position.x + sample_width;
      let z2 = position.z + sample_width;

      let ps: [Pnt3<_>; 4] = [
        hm.point_at(position.x, position.z),
        hm.point_at(position.x, z2),
        hm.point_at(x2, z2),
        hm.point_at(x2, position.z),
      ];

      let ns: [Vec3<_>; 4] = [
        hm.normal_at(normal_delta, position.x, position.z),
        hm.normal_at(normal_delta, position.x, z2),
        hm.normal_at(normal_delta, x2, z2),
        hm.normal_at(normal_delta, x2, position.z),
      ];

      let ts: [Pnt2<_>; 4] = [
        (tex_coord.to_vec() + Vec2::new(0.0, 0.0)).to_pnt(),
        (tex_coord.to_vec() + Vec2::new(0.0, tex_sample)).to_pnt(),
        (tex_coord.to_vec() + Vec2::new(tex_sample, tex_sample)).to_pnt(),
        (tex_coord.to_vec() + Vec2::new(tex_sample, 0.0)).to_pnt(),
      ];

      let center_tex_coord =
        (tex_coord.to_vec() + Pnt2::new(tex_sample, tex_sample).to_vec() / 2.0).to_pnt();

      macro_rules! place_terrain(
        ($v1: expr,
         $v2: expr,
         $minx: expr,
         $minz: expr,
         $maxx: expr,
         $maxz: expr
        ) => ({
          let v1 = &ps[$v1];
          let v2 = &ps[$v2];
          let n1 = &ns[$v1];
          let n2 = &ns[$v2];
          let t1 = &ts[$v1];
          let t2 = &ts[$v2];
          let maxy = partial_max(v1.y, v2.y);
          let maxy = maxy.and_then(|m| partial_max(m, center.y));
          let maxy = maxy.unwrap();

          let id = id_allocator.allocate();

          block.vertex_coordinates.push([*v1, *v2, center]);
          block.normals.push([*n1, *n2, center_normal]);
          block.coords.push([*t1, *t2, center_tex_coord]);
          block.ids.push(id);
          block.bounds.push((
            id,
            AABB::new(
              Pnt3::new($minx, v1.y, $minz),
              Pnt3::new($maxx, maxy, $maxz),
            ),
          ));
        });
      );

      let polys =
        (LOD_QUALITY[lod_index as usize] * LOD_QUALITY[lod_index as usize] * 4) as usize;
      block.vertex_coordinates.reserve(polys);
      block.normals.reserve(polys);
      block.coords.reserve(polys);
      block.ids.reserve(polys);
      block.bounds.reserve(polys);

      let centr = center; // makes alignment nice
      place_terrain!(0, 1, ps[0].x, ps[0].z, centr.x, ps[1].z);
      place_terrain!(1, 2, ps[1].x, centr.z, ps[2].x, ps[2].z);
      place_terrain!(2, 3, centr.x, centr.z, ps[2].x, ps[2].z);
      place_terrain!(3, 0, ps[0].x, ps[0].z, ps[3].x, centr.z);

      if treemap.should_place_tree(&centr) {
        treemap.place_tree(centr, id_allocator, block, lod_index);
      }

      true
    })
  }
}
