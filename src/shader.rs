use gl;
use gl::types::*;
use glw::gl_context::GLContext;
use glw::shader::Shader;
use std::collections::HashMap;
use std::io::fs::File;

fn split_string(s: String, pred: |char| -> bool) -> Vec<String> {
  let mut s = s;
  let mut sections = Vec::new();
  let mut cur = String::new();
  loop {
    match s.shift_char() {
      None => {
        sections.push(cur);
        return sections;
      },
      Some(c) => {
        if pred(c) {
          sections.push(cur.clone());
          cur.clear();
        } else {
          cur.push_char(c);
        }
      }
    }
  }
}

// Turn a shader definition containing $if macros into a vanilla shader
// definition, using the variable definitions provided.
// See the README in the shaders folder for details.
fn preprocess(shader: String, vars: &HashMap<String, bool>) -> Option<String> {
  let mut processed = String::new();
  let mut skipping = false;

  for line in split_string(shader, |c| c == '\n').into_iter() {
    let parts = split_string(line, |c| c == '$');
    if parts.len() == 1 {
      if !skipping {
        processed.push_str(parts[0].as_slice());
        processed.push_char('\n');
      }
    } else {
      if parts.len() != 2 {
        error!("wrong number of $ tokens on a single line");
        return None;
      }
      if !parts[0].clone().into_bytes().into_iter().all(|c| c as char == ' ') {
        error!("$ token should be preceded by only spaces");
        return None;
      }
      if parts[1] == String::from_str("else") {
        skipping = !skipping;
      } else if parts[1] == String::from_str("end") {
        skipping = false;
      } else {
        let parts = split_string(parts[1].clone(), |c| c == ' ');
        if parts.len() != 2 {
          error!("too many spaces after $if");
          return None;
        }

        if parts[0] != String::from_str("if") {
          error!("wrong token name");
          return None;
        }

        match vars.find(&parts[1]) {
          None => {
            error!("reference to undefined variable \"{}\"", parts[1]);
            return None;
          },
          Some(val) => {
            skipping = !val;
          }
        }
      }
    }
  }

  // Drop the last newline.
  let len = processed.len() - 1;
  processed.truncate(len);
  Some(processed)
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
fn preprocess_if_else() {
  let input = String::from_str(
r"
  $if foo
    foo
  $end
  $if bar
  $else
    bar
  $end
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
            (String::from_str("foo"), true),
            (String::from_str("bar"), false),
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
r"foo
$if foo
bar
$end
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
fn preprocess_undefined_command() {
  let input = String::from_str(
r"foo
$iffle foo
bar
$end
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
  vars: &HashMap<String, bool>,
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
  vars: &HashMap<String, bool>,
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
