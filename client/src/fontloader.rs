//! Font loading data structure and functions.

use std::path::Path;
use ttf;

#[allow(missing_docs)]
pub struct FontLoader {
  pub sans : ttf::Font,
  pub mono : ttf::Font,
}

impl FontLoader {
  #[allow(missing_docs)]
  pub fn new() -> FontLoader {
    FontLoader {
      sans : ttf::Font::new(&Path::new("fonts/Open_Sans/OpenSans-Regular.ttf"), 11),
      mono : ttf::Font::new(&Path::new("fonts/Ubuntu_Mono/UbuntuMono-Regular.ttf"), 11),
    }
  }
}
