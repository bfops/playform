use common::block_position::BlockPosition;
use common::entity::EntityId;
use common::id_allocator::IdAllocator;
use common::lod::LODIndex;
use common::stopwatch::TimerSet;
use common::terrain_block::{TerrainBlock, BLOCK_WIDTH, LOD_QUALITY, TEXTURE_WIDTH, tri};
use cgmath::{Point, Point2, Point3, Vector, Vector2, Vector3};
use cgmath::Aabb3;
use opencl_context::CL;
use std::cmp::partial_max;
use std::sync::Mutex;
use terrain::heightmap::HeightMap;
use terrain::texture_generator::TerrainTextureGenerator;
use terrain::tree_placer::TreePlacer;

pub fn generate_block(
  timers: &TimerSet,
  cl: &CL,
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  heightmap: &HeightMap,
  texture_generator: &TerrainTextureGenerator,
  treemap: &TreePlacer,
  position: &BlockPosition,
  lod_index: LODIndex,
) -> TerrainBlock {
  timers.time("update.generate_block", || {
    let mut block = TerrainBlock::empty();

    let position = position.to_world_position();

    let lateral_samples = LOD_QUALITY[lod_index.0 as usize] as u8;
    let sample_width = BLOCK_WIDTH as f32 / lateral_samples as f32;

    let mut any_tiles = false;
    for dx in 0 .. lateral_samples {
      let dx = dx as f32;
      for dz in 0 .. lateral_samples {
        let dz = dz as f32;
        let tex_sample =
          TEXTURE_WIDTH[lod_index.0 as usize] as f32 / lateral_samples as f32;
        let tex_coord = Point2::new(dx, dz).mul_s(tex_sample);
        let tile_position = position.add_v(&Vector3::new(dx, 0.0, dz).mul_s(sample_width));
        if add_tile(
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

fn add_tile(
  timers: &TimerSet,
  hm: &HeightMap,
  treemap: &TreePlacer,
  id_allocator: &Mutex<IdAllocator<EntityId>>,
  block: &mut TerrainBlock,
  sample_width: f32,
  tex_sample: f32,
  position: &Point3<f32>,
  tex_coord: Point2<f32>,
  lod_index: LODIndex,
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

    let ps: [Point3<_>; 4] = [
      hm.point_at(position.x, position.z),
      hm.point_at(position.x, z2),
      hm.point_at(x2, z2),
      hm.point_at(x2, position.z),
    ];

    let ns: [Vector3<_>; 4] = [
      hm.normal_at(normal_delta, position.x, position.z),
      hm.normal_at(normal_delta, position.x, z2),
      hm.normal_at(normal_delta, x2, z2),
      hm.normal_at(normal_delta, x2, position.z),
    ];

    let ts: [Point2<_>; 4] = [
      tex_coord.add_v(&Vector2::new(0.0, 0.0)),
      tex_coord.add_v(&Vector2::new(0.0, tex_sample)),
      tex_coord.add_v(&Vector2::new(tex_sample, tex_sample)),
      tex_coord.add_v(&Vector2::new(tex_sample, 0.0)),
    ];

    let center_tex_coord =
      tex_coord.add_v(&Vector2::new(tex_sample, tex_sample).mul_s(1.0 / 2.0));

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

        let id = id_allocator.lock().unwrap().allocate();

        block.vertex_coordinates.push(tri(*v1, *v2, center));
        block.normals.push(tri(*n1, *n2, center_normal));
        block.coords.push(tri(*t1, *t2, center_tex_coord));
        block.ids.push(id);
        block.bounds.push((
          id,
          Aabb3::new(
            Point3::new($minx, v1.y, $minz),
            Point3::new($maxx, maxy, $maxz),
          ),
        ));
      });
    );

    let polys =
      (LOD_QUALITY[lod_index.0 as usize] * LOD_QUALITY[lod_index.0 as usize] * 4) as usize;
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
