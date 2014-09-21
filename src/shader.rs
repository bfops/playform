use gl;
use gl::types::*;
use glw::gl_context::GLContext;
use glw::shader::Shader;
use std::collections::HashMap;
use std::io::fs::File;

// Turn a shader definition into a vanilla GLSL shader definition.
// See the README in the shaders folder for details.
fn preprocess(shader: String, vars: &HashMap<String, String>) -> Option<String> {
  let mut shader = shader;
  let mut processed = String::new();
  loop {
    match shader.shift_char() {
      None => {
        return Some(processed);
      },
      Some(c) => {
        match c {
          '$' => {
            let mut var_name = String::new();
            loop {
              match shader.shift_char() {
                None => {
                  error!("$ without matching $");
                  return None;
                },
                Some('$') => {
                  break;
                },
                Some(c) => {
                  var_name.push_char(c);
                },
              }
            }

            match vars.find(&var_name) {
              None => {
                error!("Reference to undefined variable: {}", var_name);
                return None;
              },
              Some(val) => {
                processed.push_str(val.as_slice());
              },
            }
          },
          _ => {
            processed.push_char(c);
          },
        }
      },
    }
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
        Vec::from_slice(
          [
            (String::from_str("foo"), String::from_str("foo")),
            (String::from_str("bar"), String::from_str("bar")),
          ],
        ).into_iter(),
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
fn preprocess_undefined_var() {
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

pub fn from_files<T: Iterator<(String, GLenum)>>(
  gl: &mut GLContext,
  component_paths: T,
  vars: &HashMap<String, String>,
) -> Shader {
  Shader::new(gl, component_paths.map(|(path, component_type)| {
    match File::open(&Path::new(path.as_slice())) {
      Ok(mut f) =>
        match f.read_to_string() {
          Ok(s) => {
            match preprocess(s, vars) {
              None => {
                fail!("Failed to preprocess shader \"{}\".", path);
              },
              Some(s) => (s, component_type),
            }
          },
          Err(e) => {
            fail!("Couldn't read shader file \"{}\": {}", path, e);
          }
        },
      Err(e) => {
        fail!("Couldn't open shader file \"{}\" for reading: {}", path, e);
      }
    }
  }))
}

pub fn from_file_prefix<T: Iterator<GLenum>>(
  gl: &mut GLContext,
  prefix: String,
  components: T,
  vars: &HashMap<String, String>,
) -> Shader {
  from_files(
    gl,
    components.map(|component| {
      let suffix = match component {
        gl::VERTEX_SHADER => "vert",
        gl::FRAGMENT_SHADER => "frag",
        gl::GEOMETRY_SHADER => "geom",
        _ => fail!("Unknown shader component type: {}", component),
      };
      ((prefix + "." + suffix), component)
    }),
    vars,
  )
}
