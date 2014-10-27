use common::*;
use fontloader;
use gl;
use gl::types::*;
use glw::camera;
use glw::camera::Camera;
use glw::color::Color4;
use glw::gl_buffer::*;
use glw::gl_context::GLContext;
use glw::light::Light;
use glw::queue::Queue;
use glw::shader::Shader;
use glw::texture::{Texture, TextureUnit};
use glw::vertex;
use glw::vertex::{ColoredVertex, TextureVertex};
use id_allocator::IdAllocator;
use input;
use loader::{Loader, Load};
use mob;
use nalgebra::{Pnt2, Vec2, Vec3, Pnt3, Norm};
use nalgebra::Cross;
use noise::source::Perlin;
use noise::model::Plane;
use ncollide::math::Scalar;
use ncollide::bounding_volume::aabb::AABB;
use octree;
use physics::Physics;
use player::Player;
use sdl2::mouse;
use shader;
use stopwatch;
use stopwatch::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::f32::consts::PI;
use std::iter::range_inclusive;
use std::rc::Rc;
use terrain;

static SKY_COLOR: Color4<GLfloat>  = Color4 {r: 0.2, g: 0.5, b: 0.7, a: 1.0 };

#[deriving(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Show)]
pub struct EntityId(u32);

impl Default for EntityId {
  fn default() -> EntityId {
    EntityId(0)
  }
}

impl Add<u32, EntityId> for EntityId {
  fn add(&self, rhs: &u32) -> EntityId {
    let EntityId(i) = *self;
    EntityId(i + *rhs)
  }
}

fn center(bounds: &AABB) -> Pnt3<GLfloat> {
  (bounds.mins() + bounds.maxs().to_vec()) / (2.0 as GLfloat)
}

/// Defines volumes that can be tightened.
trait TightBoundingVolume {
  /// Reduce each of a volume's bounds by some amount.
  fn tightened(&self, amount: Scalar) -> Self;
}

impl TightBoundingVolume for AABB {
  fn tightened(&self, amount: Scalar) -> AABB {
    let mut new_min = self.mins() + Vec3::new(amount, amount, amount);
    let mut new_max = self.maxs() - Vec3::new(amount, amount, amount);
    if new_min.x > new_max.x {
      let mid = (new_min.x + new_max.x) / 2.0;
      new_min.x = mid;
      new_max.x = mid;
    }
    if new_min.y > new_max.x {
      let mid = (new_min.y + new_max.x) / 2.0;
      new_min.y = mid;
      new_max.y = mid;
    }
    if new_min.z > new_max.x {
      let mid = (new_min.z + new_max.x) / 2.0;
      new_min.z = mid;
      new_max.z = mid;
    }
    AABB::new(new_min, new_max)
  }
}

fn make_text(
  gl: &GLContext,
  shader: Rc<RefCell<Shader>>,
) -> (Vec<Texture>, GLArray<TextureVertex>) {
  let fontloader = fontloader::FontLoader::new();
  let mut textures = Vec::new();
  let mut triangles = {
    GLArray::new(
      gl,
      shader,
      [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
        vertex::AttribData { name: "texture_position", size: 2, unit: vertex::Float },
      ],
      Triangles,
      GLBuffer::new(8 * VERTICES_PER_TRIANGLE),
    )
  };

  let instructions = [
          "Use WASD to move, and spacebar to jump.",
          "Use the mouse to look around, and click to remove terrain."
      ].to_vec();

  let mut y = 0.99;

  for line in instructions.iter() {
    textures.push(fontloader.sans.red(*line));

    triangles.push(
      TextureVertex::square(
        Vec2 { x: -0.97, y: y - 0.2 },
        Vec2 { x: 0.0,   y: y       }
      )
    );
    y -= 0.2;
  }

  (textures, triangles)
}

fn make_hud(
  gl: &GLContext,
  shader: Rc<RefCell<Shader>>,
) -> GLArray<ColoredVertex> {
  let mut hud_triangles = {
    GLArray::new(
      gl,
      shader,
      [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
        vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
      ],
      Triangles,
      GLBuffer::new(16 * VERTICES_PER_TRIANGLE),
    )
  };

  let cursor_color = Color4::of_rgba(0.0, 0.0, 0.0, 0.75);

  hud_triangles.push(
    ColoredVertex::square(
      Pnt2 { x: -0.02, y: -0.02 },
      Pnt2 { x:  0.02, y:  0.02 },
      cursor_color
    )
  );

  hud_triangles
}

fn make_terrain(
  physics: &mut Physics<EntityId>,
  id_allocator: &mut IdAllocator<EntityId>,
) -> (HashMap<EntityId, terrain::TerrainPiece>, Loader<EntityId, EntityId>) {
  let mut terrains = HashMap::new();
  let mut terrain_loader = Queue::new(1 << 20);

  {
    let w = 0.25;
    let place_terrain = |bounds, vertices, normal, typ| {
      place_terrain(
        physics,
        &mut terrains,
        &mut terrain_loader,
        id_allocator,
        bounds,
        vertices,
        normal,
        typ,
        false,
      );
    };

    #[allow(dead_code)]
    enum Facing {
      Up,
      Down,
      Left,
      Right,
      Front,
      Back,
    }

    let ground_steps: int = 32;
    let ground_range = (ground_steps as f32 / w) as int;

    let amplitude = 64.0;
    let perlin =
      Perlin::new()
      .seed(0)
      .frequency(1.0 / 32.0)
      .persistence(1.0 / 8.0)
      .lacunarity(8.0)
      .octaves(6)
    ;
    let plane = Plane::new(&perlin);

    // ground
    for i in range(-ground_range, ground_range) {
      for j in range(-ground_range, ground_range) {
        let at = |x, z| {
          let y = amplitude * (plane.get::<GLfloat>(x, z) + 1.0) / 2.0;
          Pnt3::new(x, y, z)
        };

        let x = i as GLfloat * w;
        let z = j as GLfloat * w;
        let center = at(x + w / 2.0, z + w / 2.0);

        let place_terrain = |typ, v1: &Pnt3<GLfloat>, v2: &Pnt3<GLfloat>, minx, minz, maxx, maxz| {
          let mut maxy = v1.y;
          if v2.y > v1.y {
            maxy = v2.y;
          }
          if center.y > maxy {
            maxy = center.y;
          }
          let side1: Vec3<GLfloat> = *center.as_vec() - *v1.as_vec();
          let side2: Vec3<GLfloat> =     *v2.as_vec() - *v1.as_vec();
          let normal: Vec3<GLfloat> = Norm::normalize_cpy(&Cross::cross(&side1, &side2));
          let bounds = AABB::new(Pnt3::new(minx, v1.y, minz), Pnt3::new(maxx, maxy, maxz));
          place_terrain(bounds, [v1.clone(), v2.clone(), center.clone()], normal, typ);
        };

        let v1 = at(x, z);
        let v2 = at(x, z + w);
        let v3 = at(x + w, z + w);
        let v4 = at(x + w, z);
        let mut center_lower_than = 0i;
        for v in [v1, v2, v3, v4].iter() {
          if center.y < v.y {
            center_lower_than += 1;
          }
        }
        let terrain =
          if center_lower_than >= 3 {
            terrain::Dirt
          } else {
            terrain::Grass
          }
        ;

        place_terrain(terrain, &v1, &v2, v1.x, v1.z, center.x, v2.z);
        place_terrain(terrain, &v2, &v3, v2.x, center.z, v3.x, v3.z);
        place_terrain(terrain, &v3, &v4, center.x, center.z, v3.x, v3.z);
        place_terrain(terrain, &v4, &v1, v1.x, v1.z, v4.x, center.z);
      }
    }

    let place_square = |x: GLfloat, y: GLfloat, z: GLfloat, dl1: GLfloat, dl2: GLfloat, typ: terrain::TerrainType, facing: Facing| {
      // Return verties such that v1 and v3 are min and max of the bounding box, respectively.
      // Vertices arranged in CCW order from the front.
      let [v1, v2, v3, v4] = match facing {
        Up => [
          Pnt3::new(x, y, z),
          Pnt3::new(x, y, z + dl2),
          Pnt3::new(x + dl1, y, z + dl2),
          Pnt3::new(x + dl1, y, z),
        ],
        Down => [
          Pnt3::new(x, y, z),
          Pnt3::new(x + dl1, y, z),
          Pnt3::new(x + dl1, y, z + dl2),
          Pnt3::new(x, y, z + dl2),
        ],
        Left => [
          Pnt3::new(x, y, z),
          Pnt3::new(x, y, z + dl1),
          Pnt3::new(x, y + dl2, z + dl1),
          Pnt3::new(x, y + dl2, z),
        ],
        Right => [
          Pnt3::new(x, y, z),
          Pnt3::new(x, y + dl2, z),
          Pnt3::new(x, y + dl2, z + dl1),
          Pnt3::new(x, y, z + dl1),
        ],
        Front => [
          Pnt3::new(x, y, z),
          Pnt3::new(x, y + dl2, z),
          Pnt3::new(x + dl1, y + dl2, z),
          Pnt3::new(x + dl1, y, z),
        ],
        Back => [
          Pnt3::new(x, y, z),
          Pnt3::new(x + dl1, y, z),
          Pnt3::new(x + dl1, y + dl2, z),
          Pnt3::new(x, y + dl2, z),
        ],
      };
      let bounds = AABB::new(v1, v3);
      let normal = Norm::normalize_cpy(&Cross::cross(&(v2.as_vec() - v1.as_vec().clone()), &(v3.as_vec() - v2.as_vec().clone())));
      place_terrain(bounds, [v1, v2, v4], normal, typ);
      place_terrain(bounds, [v2, v3, v4], normal, typ);
    };

    let wall_height = (32.0 / w) as int;
    // front wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(i, j, -ground_steps as f32, w, w, terrain::Stone, Back);
      }
    }
    // back wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(i, j, ground_steps as f32 - w, w, w, terrain::Stone, Front);
      }
    }
    // left wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(-ground_steps as f32, j, i, w, w, terrain::Stone, Right);
      }
    }
    // right wall
    for i in range_inclusive(-ground_range, ground_range) {
      for j in range_inclusive(0i, wall_height) {
        let (i, j) = (i as GLfloat * w, j as GLfloat * w);
        place_square(ground_steps as f32 - w, j, i, w, w, terrain::Stone, Left);
      }
    }
  }

  (terrains, terrain_loader)
}

/// The whole application. Wrapped up in a nice frameworky struct for piston.
pub struct App<'a> {
  pub physics: Physics<EntityId>,
  pub terrains: HashMap<EntityId, terrain::TerrainPiece>,
  pub player: Player,
  pub mobs: HashMap<EntityId, mob::Mob>,

  pub terrain_loader: Loader<EntityId, EntityId>,
  pub octree_loader: Rc<RefCell<Loader<(octree::OctreeId, AABB), octree::OctreeId>>>,

  // OpenGL buffers
  pub mob_buffers: mob::MobBuffers,
  pub terrain_buffers: terrain::TerrainBuffers,
  pub octree_buffers: octree::OctreeBuffers<EntityId>,
  pub line_of_sight: GLArray<ColoredVertex>,
  pub hud_triangles: GLArray<ColoredVertex>,
  pub text_triangles: GLArray<TextureVertex>,

  pub misc_texture_unit: TextureUnit,
  pub text_textures: Vec<Texture>,

  // OpenGL shader "program" ids
  pub color_shader: Rc<RefCell<Shader>>,
  pub texture_shader: Rc<RefCell<Shader>>,
  pub hud_texture_shader: Rc<RefCell<Shader>>,
  pub hud_color_shader: Rc<RefCell<Shader>>,

  // which mouse buttons are currently pressed
  pub mouse_buttons_pressed: Vec<input::mouse::Button>,

  pub render_octree: bool,
  pub render_outlines: bool,

  pub timers: Rc<stopwatch::TimerSet>,
  pub gl: GLContext,
}

impl<'a> App<'a> {
  /// Initializes an empty app.
  pub fn new() -> App<'a> {
    let timers = Rc::new(stopwatch::TimerSet::new());
    time!(timers.deref(), "load", || {
      let mut gl = GLContext::new();

      gl.print_stats();

      gl::FrontFace(gl::CCW);
      gl::CullFace(gl::BACK);
      gl::Enable(gl::CULL_FACE);
      gl.enable_alpha_blending();
      gl.enable_smooth_lines();
      gl.enable_depth_buffer(100.0);
      gl.set_background_color(SKY_COLOR);
      mouse::show_cursor(false);

      let world_bounds = AABB::new(
        Pnt3 { x: -512.0, y: -32.0, z: -512.0 },
        Pnt3 { x: 512.0, y: 512.0, z: 512.0 },
      );

      let texture_shader = {
        let texture_shader =
          Rc::new(RefCell::new(shader::from_file_prefix(
            &mut gl,
            String::from_str("shaders/world_texture"),
            [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
            &FromIterator::from_iter(
              [(String::from_str("lighting"), (USE_LIGHTING as uint).to_string())].to_vec().into_iter(),
            ),
          )));
        if USE_LIGHTING {
          texture_shader.borrow_mut().set_point_light(
            &mut gl,
            &Light {
              position: Vec3::new(0.0, 16.0, 0.0),
              intensity: Vec3::new(0.6, 0.6, 0.6),
            }
          );
          texture_shader.borrow_mut().set_ambient_light(
            &mut gl,
            Vec3::new(0.4, 0.4, 0.4),
          );
          texture_shader.borrow_mut().set_ambient_light(
            &mut gl,
            Vec3::new(0.4, 0.4, 0.4),
          );
        }
        texture_shader
      };
      let color_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/color"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &HashMap::new(),
        )));
      let hud_color_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/color"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &HashMap::new(),
        )));
      let hud_texture_shader =
        Rc::new(RefCell::new(shader::from_file_prefix(
          &mut gl,
          String::from_str("shaders/hud_texture"),
          [ gl::VERTEX_SHADER, gl::FRAGMENT_SHADER, ].to_vec().into_iter(),
          &HashMap::new(),
        )));

      {
        let hud_camera = {
          let mut c = Camera::unit();
          c.fov = camera::sortho(WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32, 1.0, -1.0, 1.0);
          c.fov = camera::translation(Vec3::new(0.0, 0.0, -1.0)) * c.fov;
          c
        };

        hud_color_shader.borrow_mut().set_camera(&mut gl, &hud_camera);
        hud_texture_shader.borrow_mut().set_camera(&mut gl, &hud_camera);
      }

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} setting up shaders", err),
      }

      let line_of_sight = {
        let mut line_of_sight = {
          GLArray::new(
            &gl,
            color_shader.clone(),
            [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
              vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
            ],
            Lines,
            GLBuffer::new(2 * 2),
          )
        };

        line_of_sight.push(
          [
            ColoredVertex {
              position: Pnt3::new(0.0, 0.0, 0.0),
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
            ColoredVertex {
              position: Pnt3::new(0.0, 0.0, 0.0),
              color: Color4::of_rgba(1.0, 0.0, 0.0, 1.0),
            },
          ]
        );

        line_of_sight
      };

      let hud_triangles = make_hud(&gl, hud_color_shader.clone());

      let octree_loader = Rc::new(RefCell::new(Queue::new(4 * MAX_WORLD_SIZE)));

      let octree_buffers = unsafe {
        octree::OctreeBuffers::new(&gl, &color_shader)
      };

      let mut texture_unit_alloc: IdAllocator<TextureUnit> = IdAllocator::new();

      let terrain_buffers = {
        let terrain_buffers = terrain::TerrainBuffers::new(&gl);
        terrain_buffers.bind(&mut gl, &mut texture_unit_alloc, texture_shader.clone());
        terrain_buffers
      };

      let (text_textures, text_triangles) = make_text(&gl, hud_texture_shader.clone());

      let mut physics =
        Physics {
          octree: octree::Octree::new(octree_loader.clone(), &world_bounds),
          bounds: HashMap::new(),
        };

      let mut id_allocator = IdAllocator::new();

      let (terrains, terrain_loader) =
        time!(timers.deref(), "make_terrain", || {
          make_terrain(
            &mut physics,
            &mut id_allocator,
          )
        });

      let (mobs, mob_buffers) =
        time!(timers.deref(), "make_mobs", || {
          make_mobs(
            &gl,
            &mut physics,
            &mut id_allocator,
            color_shader.clone(),
          )
        });

      let player = {
        let mut player = Player {
          camera: Camera::unit(),
          speed: Vec3::new(0.0, 0.0, 0.0),
          accel: Vec3::new(0.0, -0.1, 0.0),
          walk_accel: Vec3::new(0.0, 0.0, 0.0),
          jump_fuel: 0,
          is_jumping: false,
          id: id_allocator.allocate(),
          lateral_rotation: 0.0,
          vertical_rotation: 0.0,
        };

        let min = Pnt3::new(0.0, 64.0, 4.0);
        let max = min + Vec3::new(1.0, 2.0, 1.0);
        let bounds = AABB::new(min, max);
        physics.insert(player.id, &bounds);

        // initialize the projection matrix
        player.camera.translate(center(&bounds).to_vec());
        player.camera.fov = camera::perspective(3.14/3.0, 4.0/3.0, 0.1, 100.0);
        player.rotate_lateral(PI / 2.0);

        player
      };

      let misc_texture_unit = texture_unit_alloc.allocate();
      gl::ActiveTexture(misc_texture_unit.gl_id());
      hud_texture_shader.borrow_mut().with_uniform_location(
        &mut gl,
        "texture_in",
        |loc| {
          gl::Uniform1i(loc, misc_texture_unit.glsl_id as GLint);
        }
      );

      match gl::GetError() {
        gl::NO_ERROR => {},
        err => fail!("OpenGL error 0x{:x} in load()", err),
      }

      debug!("load() finished with {} terrain polys", terrains.len());

      App {
        line_of_sight: line_of_sight,
        physics: physics,
        terrain_loader: terrain_loader,
        octree_loader: octree_loader,
        mob_buffers: mob_buffers,
        octree_buffers: octree_buffers,
        terrain_buffers: terrain_buffers,
        terrains: terrains,
        player: player,
        mobs: mobs,
        hud_triangles: hud_triangles,
        text_textures: text_textures,
        text_triangles: text_triangles,
        misc_texture_unit: misc_texture_unit,
        color_shader: color_shader,
        texture_shader: texture_shader,
        hud_color_shader: hud_color_shader,
        hud_texture_shader: hud_texture_shader,
        mouse_buttons_pressed: Vec::new(),
        render_octree: false,
        render_outlines: false,
        timers: timers.clone(),
        gl: gl,
      }
    })
  }

  #[inline]
  pub fn is_mouse_pressed(&self, b: input::mouse::Button) -> bool {
    self.mouse_buttons_pressed.iter().any(|x| *x == b)
  }

  fn get_bounds(&self, id: EntityId) -> &AABB {
    self.physics.get_bounds(id).unwrap()
  }

  fn add_mob(
    physics: &mut Physics<EntityId>,
    mobs: &mut HashMap<EntityId, mob::Mob>,
    mob_buffers: &mut mob::MobBuffers,
    id_allocator: &mut IdAllocator<EntityId>,
    low_corner: Pnt3<GLfloat>,
    behavior: mob::Behavior,
  ) {
    // TODO: mob loader instead of pushing directly to gl buffers

    let id = id_allocator.allocate();

    let mob =
      mob::Mob {
        speed: Vec3::new(0.0, 0.0, 0.0),
        behavior: behavior,
        id: id,
      };

    let bounds = AABB::new(low_corner, low_corner + Vec3::new(1.0, 2.0, 1.0 as GLfloat));
    mob_buffers.push(id, to_triangles(&bounds, &Color4::of_rgba(1.0, 0.0, 0.0, 1.0)));

    physics.insert(id, &bounds);
    mobs.insert(id, mob);
  }
}

fn place_terrain(
  physics: &mut Physics<EntityId>,
  terrains: &mut HashMap<EntityId, terrain::TerrainPiece>,
  terrain_loader: &mut Loader<EntityId, EntityId>,
  id_allocator: &mut IdAllocator<EntityId>,
  bounds: AABB,
  vertices: [Pnt3<GLfloat>, ..3],
  normal: Vec3<GLfloat>,
  typ: terrain::TerrainType,
  check_collisions: bool,
) {
  // hacky solution to make sure terrain polys have "breathing room" and don't
  // collide with their neighbours.
  let epsilon: GLfloat = 0.00001;
  if !(check_collisions && physics.octree.intersect(&bounds.tightened(epsilon), None).is_some()) {
    let terrain = terrain::TerrainPiece {
      vertices: vertices,
      normal: normal,
      typ: typ as GLuint,
      id: id_allocator.allocate(),
    };
    physics.insert(terrain.id, &bounds);
    terrains.insert(terrain.id, terrain);
    terrain_loader.push(Load(terrain.id));
  }
}

fn make_mobs(
  gl: &GLContext,
  physics: &mut Physics<EntityId>,
  id_allocator: &mut IdAllocator<EntityId>,
  shader: Rc<RefCell<Shader>>,
) -> (HashMap<EntityId, mob::Mob>, mob::MobBuffers) {
  let mut mobs = HashMap::new();
  let mut mob_buffers = mob::MobBuffers::new(gl, shader);

  fn mob_behavior(world: &App, mob: &mut mob::Mob) {
    let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
    if Norm::norm(&to_player) < 2.0 {
      mob.behavior = wait_for_distance;
    }

    fn wait_for_distance(world: &App, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if Norm::norm(&to_player) > 8.0 {
        mob.behavior = follow_player;
      }
    }

    fn follow_player(world: &App, mob: &mut mob::Mob) {
      let mut to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if to_player.normalize() < 2.0 {
        mob.behavior = wait_to_reset;
        mob.speed = Vec3::new(0.0, 0.0, 0.0);
      } else {
        mob.speed = to_player / 2.0 as GLfloat;
      }
    }

    fn wait_to_reset(world: &App, mob: &mut mob::Mob) {
      let to_player = center(world.get_bounds(world.player.id)) - center(world.get_bounds(mob.id));
      if Norm::norm(&to_player) >= 2.0 {
        mob.behavior = mob_behavior;
      }
    }
  }

  App::add_mob(
    physics,
    &mut mobs,
    &mut mob_buffers,
    id_allocator,
    Pnt3::new(0.0, 64.0, -1.0),
    mob_behavior
  );

  (mobs, mob_buffers)
}

// TODO(cgaebel): This should be removed when rustc bug #8861 is patched.
#[unsafe_destructor]
impl<'a> Drop for App<'a> {
  fn drop(&mut self) {
    info!("Update Stats");
    info!("====================");
    self.timers.print();
  }
}
