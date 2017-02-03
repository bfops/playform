//! The state associated with perceiving the world state.

pub mod camera;
pub mod chunk_stats;
pub mod grass_buffers;
pub mod light;
pub mod mob_buffers;
pub mod player_buffers;
mod render;
pub mod shaders;
pub mod terrain_buffers;
pub mod thread;
pub mod update;

pub use self::render::render;

use cgmath;
use gl;
use gl::types::*;
use image;
use image::GenericImage;
use std;
use yaglw::gl_context::GLContext;
use yaglw;
use yaglw::vertex_buffer::{GLArray, GLBuffer, GLType, DrawMode, VertexAttribData};
use yaglw::texture::{TextureUnit};

use common::id_allocator;
use vertex::{ColoredVertex};

/// FOV in radians
pub const FOV: f32 = std::f32::consts::FRAC_PI_3;

const VERTICES_PER_TRIANGLE: usize = 3;

#[allow(missing_docs)]
pub enum InputMode {
  Camera,
  Sun,
}

/// The state associated with perceiving the world state.
pub struct T<'a> {
  /// Current OpengL context.
  pub gl: GLContext,

  #[allow(missing_docs)]
  pub shaders: shaders::T<'a>,
  #[allow(missing_docs)]
  pub empty_gl_array: yaglw::vertex_buffer::ArrayHandle<'a>,
  /// A texture unit for misc use.
  pub misc_texture_unit: TextureUnit,
  /// The OpenGL buffers for terrain render data
  pub terrain_buffers: terrain_buffers::T<'a>,
  /// The OpenGL buffers for grass render data
  pub grass_buffers: grass_buffers::T<'a>,
  /// The OpenGL texture to sample for grass
  pub grass_texture: yaglw::texture::Texture2D<'a>,
  /// OpenGL buffers for mob render data
  pub mob_buffers: mob_buffers::T<'a>,
  /// OpenGL buffers for player render data
  pub player_buffers: player_buffers::T<'a>,
  /// Hud triangles for non-text.
  pub hud_triangles: GLArray<'a, ColoredVertex>,

  #[allow(missing_docs)]
  pub sun: light::Sun,
  #[allow(missing_docs)]
  pub camera: camera::T,
  #[allow(missing_docs)]
  pub window_size: cgmath::Vector2<i32>,
  /// Whether to render HUD elements
  pub show_hud: bool,

  /// Whether to render HUD elements
  pub input_mode: InputMode,

  /// Distance to near clip plane
  pub near_clip: f32,
  /// Distance to far clip plane
  pub far_clip: f32,
}

fn load_grass_texture<'a, 'b:'a>(
  gl: &'a mut GLContext,
) -> image::ImageResult<yaglw::texture::Texture2D<'b>> {
  let grass_texture = yaglw::texture::Texture2D::new(&gl);
  let fd = std::fs::File::open("textures/Free_Vector_Grass.png").unwrap();
  let buffered_file = std::io::BufReader::new(fd);
  let image = try!(image::load(buffered_file, image::ImageFormat::PNG));
  let (w, h) = image.dimensions();
  let colortype = image.color();
  assert!(colortype == image::ColorType::RGBA(8));

  let mut pixels: Vec<u8> = Vec::with_capacity((w * h) as usize);
  for y in 0 .. h {
  for x in 0 .. w {
    let y = h - y - 1;
    let pixel = image.get_pixel(x, y);
    pixels.extend_from_slice(&pixel.data);
  }}

  unsafe {
    gl::BindTexture(gl::TEXTURE_2D, grass_texture.handle.gl_id);
    let data = std::mem::transmute(pixels.as_ptr());
    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32, w as i32, h as i32, 0, gl::RGBA, gl::UNSIGNED_BYTE, data);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
  }

  Ok(grass_texture)
}

#[allow(missing_docs)]
pub fn new<'a>(
  mut gl: GLContext,
  window_size: cgmath::Vector2<i32>,
) -> T<'a> {
  let mut texture_unit_alloc = id_allocator::new();

  let mut shaders = shaders::new(&mut gl, window_size);

  let terrain_buffers = terrain_buffers::new(&mut gl);
  terrain_buffers.bind_vertex_positions(
    &mut gl,
    &mut texture_unit_alloc,
    &mut shaders.terrain_shader.shader,
  );
  terrain_buffers.bind_normals(
    &mut gl,
    &mut texture_unit_alloc,
    &mut shaders.terrain_shader.shader,
  );
  terrain_buffers.bind_materials(
    &mut gl,
    &mut texture_unit_alloc,
    &mut shaders.terrain_shader.shader,
  );
  terrain_buffers.bind_vertex_positions(
    &mut gl,
    &mut texture_unit_alloc,
    &mut shaders.grass_billboard.shader,
  );
  terrain_buffers.bind_normals(
    &mut gl,
    &mut texture_unit_alloc,
    &mut shaders.grass_billboard.shader,
  );

  let mob_buffers = mob_buffers::new(&mut gl, &shaders.mob_shader);
  let player_buffers = player_buffers::new(&mut gl, &shaders.mob_shader);

  let buffer = GLBuffer::new(&mut gl, 16 * VERTICES_PER_TRIANGLE);
  let hud_triangles = {
    GLArray::new(
      &mut gl,
      &shaders.hud_color_shader.shader,
      &[
        VertexAttribData { name: "position", size: 3, unit: GLType::Float, divisor: 0 },
        VertexAttribData { name: "in_color", size: 4, unit: GLType::Float, divisor: 0 },
      ],
      DrawMode::Triangles,
      buffer,
    )
  };

  let misc_texture_unit = texture_unit_alloc.allocate();

  unsafe {
    gl::FrontFace(gl::CCW);
    gl::CullFace(gl::BACK);
    gl::Enable(gl::CULL_FACE);

    gl::Enable(gl::BLEND);
    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

    gl::Enable(gl::LINE_SMOOTH);
    gl::LineWidth(2.5);

    gl::Enable(gl::DEPTH_TEST);
    gl::DepthFunc(gl::LESS);
    gl::ClearDepth(1.0);
  }

  unsafe {
    gl::ActiveTexture(misc_texture_unit.gl_id());
  }

  let texture_in =
    shaders.texture_shader.shader.get_uniform_location("texture_in");
  shaders.texture_shader.shader.use_shader(&mut gl);
  unsafe {
    gl::Uniform1i(texture_in, misc_texture_unit.glsl_id as GLint);
  }

  let texture_in =
    shaders.grass_billboard.shader.get_uniform_location("texture_in");
  shaders.grass_billboard.shader.use_shader(&mut gl);
  unsafe {
    gl::Uniform1i(texture_in, misc_texture_unit.glsl_id as GLint);
  }

  let grass_buffers = grass_buffers::new(&mut gl, &shaders.grass_billboard.shader);
  let grass_texture = load_grass_texture(&mut gl).unwrap();

  let empty_gl_array = yaglw::vertex_buffer::ArrayHandle::new(&gl);

  let near_clip = 0.1;
  let far_clip = 2048.0;

  T {
    gl: gl,
    shaders: shaders,

    terrain_buffers: terrain_buffers,
    grass_buffers: grass_buffers,
    grass_texture: grass_texture,
    mob_buffers: mob_buffers,
    player_buffers: player_buffers,
    hud_triangles: hud_triangles,

    empty_gl_array: empty_gl_array,
    misc_texture_unit: misc_texture_unit,

    window_size: window_size,

    camera: {
      let fovy = cgmath::Rad(FOV);
      let aspect = window_size.x as f32 / window_size.y as f32;
      let mut camera = camera::unit();
      // Initialize the projection matrix.
      camera.fov = cgmath::perspective(fovy, aspect, near_clip, far_clip);
      // TODO: This should use player rotation from the server.
      camera.rotate_lateral(std::f32::consts::PI / 2.0);
      camera
    },

    sun:
      light::Sun {
        progression: 0.0,
        rotation: 0.0,
      },

    show_hud: true,
    input_mode: InputMode::Camera,

    near_clip: near_clip,
    far_clip: far_clip,
  }
}
