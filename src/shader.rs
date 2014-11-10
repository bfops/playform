use gl;
use gl::types::*;
use glw::gl_context::GLContextExistence;
use glw::shader::Shader;
use std::collections::HashMap;
use std::io::fs::File;

// Turn a shader definition into a vanilla GLSL shader definition.
// See the README in the shaders folder for details.
fn preprocess(shader: String, vars: &HashMap<String, String>) -> Option<String> {
  let shader = shader.into_bytes();
  let mut processed = String::new();
  let mut i = 0;

  // For termination conditions, look for explicit returns.
  loop {
    if i >= shader.len() {
      return Some(processed);
    }
    match shader[i] as char {
      '$' => {
        let mut var_name = String::new();
        loop {
          i += 1;
          if i >= shader.len() {
            error!("$ without matching $");
            return None;
          }
          match shader[i] as char {
            '$' => {
              break;
            },
            c => {
              var_name.push(c);
            },
          }
        }

        match vars.get(&var_name) {
          None => {
            error!("Reference to undefined variable: {}", var_name);
            return None;
          },
          Some(val) => {
            processed.push_str(val.as_slice());
          },
        }
      },
      c => {
        processed.push(c);
      },
    }

    i += 1;
  }
}

#[test]
fn preprocess_vanilla_shader() {
  let input = String::from_str(
r"foo
bar
baz
");
  let expected = Some(String::from_str(
r"foo
bar
baz
"));
  let actual = preprocess(input, &HashMap::new());
  assert!(
    actual == expected,
    "{} != {}",
    actual,
    expected
  );
}

#[test]
fn preprocess_var() {
  let input = String::from_str(
r"
$foo$
$bar$
baz
");
  let expected = Some(String::from_str(
r"
foo
bar
baz
"));
  let actual =
    preprocess(
      input,
      &FromIterator::from_iter(
        [
          (String::from_str("foo"), String::from_str("foo")),
          (String::from_str("bar"), String::from_str("bar")),
        ].to_vec().into_iter(),
      ),
    );
  assert!(
    actual == expected,
    "{} != {}",
    actual,
    expected
  );
}

#[test]
fn preprocess_no_matching_token() {
  let input = String::from_str(
r"
$foo
bar
baz
");
  let expected = None;
  let actual = preprocess(input, &HashMap::new());
  assert!(
    actual == expected,
    "{} != {}",
    actual,
    expected
  );
}

#[test]
fn preprocess_undefined_var() {
  let input = String::from_str(
r"
$foo$
bar
baz
");
  let expected = None;
  let actual = preprocess(input, &HashMap::new());
  assert!(
    actual == expected,
    "{} != {}",
    actual,
    expected
  );
}

pub fn from_files<'a, T: Iterator<(String, GLenum)>>(
  gl: &'a GLContextExistence,
  component_paths: T,
  vars: &HashMap<String, String>,
) -> Shader<'a> {
  Shader::new(gl, component_paths.map(|(path, component_type)| {
    match File::open(&Path::new(path.as_slice())) {
      Ok(mut f) =>
        match f.read_to_string() {
          Ok(s) => {
            match preprocess(s, vars) {
              None => {
                panic!("Failed to preprocess shader \"{}\".", path);
              },
              Some(s) => (s, component_type),
            }
          },
          Err(e) => {
            panic!("Couldn't read shader file \"{}\": {}", path, e);
          }
        },
      Err(e) => {
        panic!("Couldn't open shader file \"{}\" for reading: {}", path, e);
      }
    }
  }))
}

pub fn from_file_prefix<'a, T: Iterator<GLenum>>(
  gl: &'a GLContextExistence,
  prefix: String,
  components: T,
  vars: &HashMap<String, String>,
) -> Shader<'a> {
  from_files(
    gl,
    components.map(|component| {
      let suffix =
        match component {
          gl::VERTEX_SHADER => "vert",
          gl::FRAGMENT_SHADER => "frag",
          gl::GEOMETRY_SHADER => "geom",
          _ => panic!("Unknown shader component type: {}", component),
        }
      ;
      ((prefix + "." + suffix), component)
    }),
    vars,
  )
}
